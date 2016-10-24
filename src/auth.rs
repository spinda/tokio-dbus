use futures::{self, Async, BoxFuture, Future, Poll};
use libc;
use nom::IResult;
use std::borrow::Cow;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{Error, ErrorKind};
use std::str;
use tokio_core::easy::{EasyBuf, EasyFramed, Parse, Serialize};
use tokio_core::io::Io;

use framed;

pub enum AuthMode {
    External,
}

pub type AuthFuture<T> = BoxFuture<T, AuthError>;

#[derive(Debug)]
pub enum AuthError {
    Io(Error),
    Rejected,
    ServerError,
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            AuthError::Io(ref err) => write!(f, "D-Bus IO error: {}", err),
            _ => f.write_str(error::Error::description(self)),
        }
    }
}

impl error::Error for AuthError {
    fn description(&self) -> &str {
        match *self {
            AuthError::Io(ref err) => err.description(),
            AuthError::Rejected => "D-Bus server rejected the authentication attempt",
            AuthError::ServerError => "D-Bus server reported an authentication error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            AuthError::Io(ref err) => Some(err),
            AuthError::ServerError | AuthError::Rejected => None,
        }
    }
}

impl From<Error> for AuthError {
    fn from(err: Error) -> Self {
        AuthError::Io(err)
    }
}

impl From<AuthError> for Error {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Io(err) => err,
            AuthError::Rejected => {
                Error::new(ErrorKind::PermissionDenied, error::Error::description(&err))
            }
            AuthError::ServerError => {
                Error::new(ErrorKind::InvalidInput, error::Error::description(&err))
            }
        }
    }
}

pub fn authenticate<T>(upstream: T, auth_mode: AuthMode) -> AuthFuture<T>
    where T: Io + Send + 'static
{
    let auth_stream = AuthStream::new(upstream);
    let auth = match auth_mode {
        AuthMode::External => auth_external(auth_stream),
    };
    let pack = auth.map(AuthStream::into_upstream);

    pack.boxed()
}

fn get_uid_str() -> String {
    unsafe { libc::getuid().to_string() }
}

fn auth_external<T>(auth_stream: AuthStream<T>) -> AuthFuture<AuthStream<T>>
    where T: Io + Send + 'static
{
    let send = auth_stream.send(ClientCommand::Auth {
        mechanism: "EXTERNAL".into(),
        initial_response: Some(get_uid_str().into()),
    });
    let recv = send.and_then(|auth_stream| auth_stream.recv());
    let begin = recv.and_then(|(auth_stream, cmd)| {
        match cmd { 
            ServerCommand::Ok => auth_stream.send(ClientCommand::Begin),
            ServerCommand::Rejected => futures::failed(AuthError::Rejected).boxed(),
            ServerCommand::Error => futures::failed(AuthError::ServerError).boxed(),
            // _ => futures::failed(AuthError::from(unexpected_response())).boxed(),
        }
    });

    begin.boxed()
}

#[derive(Debug)]
enum ClientCommand {
    Auth {
        mechanism: Cow<'static, str>,
        initial_response: Option<Cow<'static, str>>,
    },
    Begin,
}

#[derive(Debug)]
enum ServerCommand {
    Ok,
    Rejected,
    Error,
}

struct AuthStream<T> {
    inner: EasyFramed<T, ServerCommandParser, ClientCommandSerializer>,
}

impl<T> AuthStream<T>
    where T: Io + Send + 'static
{
    pub fn new(upstream: T) -> Self {
        AuthStream {
            inner: EasyFramed::new(upstream, ServerCommandParser, ClientCommandSerializer),
        }
    }

    pub fn into_upstream(self) -> T {
        self.inner.into_upstream()
    }

    pub fn recv(self) -> AuthFuture<(Self, ServerCommand)> {
        let AuthStream { inner } = self;

        let read = framed::read(inner);
        let unwrap = read.and_then(|(inner, cmd)| {
            match cmd {
                Some(cmd) => Ok((inner, cmd)),
                None => Err(connection_reset()),
            }
        });
        let pack = unwrap.map(|(inner, cmd)| (AuthStream { inner: inner }, cmd))
            .map_err(AuthError::from);

        pack.boxed()
    }

    pub fn send(self, cmd: ClientCommand) -> AuthFuture<Self> {
        let AuthStream { inner } = self;

        let write = framed::write(inner, cmd);
        let flush = write.and_then(framed::flush);
        let pack = flush.map(|inner| AuthStream { inner: inner }).map_err(AuthError::from);

        pack.boxed()
    }
}

struct ClientCommandSerializer;

static HEX_CHARS: &'static [u8] = b"0123456789abcdef";

fn hex_encoded_len(src: &str) -> usize {
    2 * src.len()
}

fn extend_from_hex_encoded(buf: &mut Vec<u8>, src: &str) {
    for byte in src.as_bytes() {
        buf.push(HEX_CHARS[(byte >> 4) as usize]);
        buf.push(HEX_CHARS[(byte & 0xf) as usize]);
    }
}

impl Serialize for ClientCommandSerializer {
    type In = ClientCommand;

    fn serialize(&mut self, msg: &Self::In, buf: &mut Vec<u8>) {
        match *msg {
            ClientCommand::Auth { ref mechanism, ref initial_response } => {
                buf.reserve(8 + initial_response.as_ref().map_or(0, |s| 1 + hex_encoded_len(s)));

                buf.extend_from_slice(b"\0AUTH ");
                buf.extend_from_slice(mechanism.as_bytes());

                match *initial_response {
                    None => {}
                    Some(ref initial_response) => {
                        buf.push(b' ');
                        extend_from_hex_encoded(buf, initial_response);
                    }
                }

                buf.extend_from_slice(b"\r\n");
            }
            ClientCommand::Begin => buf.extend_from_slice(b"BEGIN\r\n"),
        }
    }
}

struct ServerCommandParser;

named!(parse_server_cmd<&[u8], ServerCommand>,
    chain!(
        cmd: alt!(parse_server_cmd_ok | parse_server_cmd_rejected | parse_server_cmd_error) ~
        tag!("\r\n"),
        || { cmd }
    )
);

named!(parse_server_cmd_ok<&[u8], ServerCommand>,
    value!(ServerCommand::Ok, preceded!(tag!("OK "), take_until!("\r\n")))
);

named!(parse_server_cmd_rejected<&[u8], ServerCommand>,
    value!(ServerCommand::Rejected, preceded!(tag!("REJECTED "), take_until!("\r\n")))
);

named!(parse_server_cmd_error<&[u8], ServerCommand>,
    value!(ServerCommand::Error, tag!("ERROR"))
);

impl Parse for ServerCommandParser {
    type Out = ServerCommand;

    fn parse(&mut self, buf: &mut EasyBuf) -> Poll<Self::Out, Error> {
        let (result, consumed) = match parse_server_cmd(buf.as_slice()) {
            IResult::Done(remaining, cmd) => (Ok(Async::Ready(cmd)), buf.len() - remaining.len()),
            IResult::Incomplete(_) => return Ok(Async::NotReady),
            IResult::Error(_) => return Err(unexpected_response()),
        };
        buf.drain_to(consumed);
        result
    }
}

fn connection_reset() -> Error {
    Error::new(ErrorKind::ConnectionReset, "D-Bus connection reset")
}

fn unexpected_response() -> Error {
    Error::new(ErrorKind::InvalidData, "unexpected D-Bus auth response")
}
