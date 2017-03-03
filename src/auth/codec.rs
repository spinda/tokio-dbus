use nom::{self, IResult, Needed};
use std::io::{Error, ErrorKind, Result};
use tokio_core::io::{Codec, EasyBuf};

use auth::types::{ClientCommand, ServerCommand, ServerGuid};

pub struct AuthCodec;

impl Codec for AuthCodec {
    type In = ServerCommand;
    type Out = ClientCommand;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>> {
        let (result, consumed) = match parse_server_cmd(buf.as_slice()) {
            IResult::Done(remaining, cmd) => (Ok(Some(cmd)), buf.len() - remaining.len()),
            IResult::Incomplete(_) => return Ok(None),
            IResult::Error(_) => {
                return Err(Error::new(ErrorKind::InvalidData,
                                      "unable to parse auth response from D-Bus server"))
            }
        };
        buf.drain_to(consumed);
        result
    }

    fn encode(&mut self, cmd: Self::Out, buf: &mut Vec<u8>) -> Result<()> {
        match cmd {
            ClientCommand::Auth { mechanism, initial_response } => {
                match initial_response {
                    None => {
                        buf.reserve_exact(5 + mechanism.len() + 2);

                        buf.extend_from_slice(b"AUTH ");
                        // ^ 5 bytes
                        buf.extend_from_slice(&mechanism);
                        // ^ mechanism.len() bytes
                        buf.extend_from_slice(b"\r\n");
                        // ^ 2 bytes
                    }
                    Some(initial_response) => {
                        buf.reserve_exact(5 + mechanism.len() + 1 +
                                          hex_encoded_len(&initial_response) +
                                          2);

                        buf.extend_from_slice(b"AUTH ");
                        // ^ 5 bytes
                        buf.extend_from_slice(&mechanism);
                        // ^ mechanism.len() bytes
                        buf.push(b' ');
                        // ^ 1 byte
                        extend_from_hex_encoded(buf, &initial_response);
                        // ^ hex_encoded_len(&initial_response) bytes
                        buf.extend_from_slice(b"\r\n");
                        // ^ 2 bytes
                    }
                }
            }
            ClientCommand::Begin => buf.extend_from_slice(b"BEGIN\r\n"),
            ClientCommand::Cancel => buf.extend_from_slice(b"CANCEL\r\n"),
            ClientCommand::Data(bytes) => {
                buf.reserve_exact(5 + hex_encoded_len(&bytes) + 2);

                buf.extend_from_slice(b"DATA ");
                // ^ 5 bytes
                extend_from_hex_encoded(buf, &bytes);
                // ^ hex_encoded_len(&bytes) bytes
                buf.extend_from_slice(b"\r\n");
                // ^ 2 bytes
            }
            ClientCommand::Error(message) => {
                match message {
                    None => buf.extend_from_slice(b"ERROR\r\n"),
                    Some(message) => {
                        buf.reserve_exact(6 + message.len() + 2);

                        buf.extend_from_slice(b"ERROR ");
                        // ^ 6 bytes
                        buf.extend_from_slice(&message);
                        // ^ message.len() bytes
                        buf.extend_from_slice(b"\r\n");
                        // ^ 2 bytes
                    }
                }
            }
            ClientCommand::Raw { cmd, payload } => {
                match payload {
                    None => buf.extend_from_slice(&cmd),
                    Some(payload) => {
                        buf.reserve_exact(cmd.len() + 1 + payload.len() + 2);

                        buf.extend_from_slice(&cmd);
                        // ^ cmd.len() bytes
                        buf.push(b' ');
                        // ^ 1 byte
                        buf.extend_from_slice(&payload);
                        // ^ payload.len() bytes
                        buf.extend_from_slice(b"\r\n");
                        // ^ 2 bytes
                    }
                }
            }
        }

        Ok(())
    }
}

macro_rules! hex_uint(
    ($input:expr, $typ:ty, $n:expr) => (
        {
            if $input.len() < $n {
                IResult::Incomplete::<_, _>(Needed::Size($n))
            } else {
                let mut acc: $typ = 0;
                for c in &$input[0..$n] {
                    acc <<= 4;
                    match *c {
                        c @ b'0' ... b'9' => acc += (c - b'0') as $typ,
                        c @ b'a' ... b'f' => acc += (c - b'a' + 10) as $typ,
                        _ => return IResult::Error(nom::ErrorKind::HexDigit)
                    }
                }
                IResult::Done(&$input[$n..], acc)
            }
        }
    )
);

named!(parse_server_cmd(&[u8]) -> ServerCommand,
    chain!(
        cmd: alt!(
            parse_server_cmd_data |
            parse_server_cmd_error |
            parse_server_cmd_ok |
            parse_server_cmd_rejected |
            parse_server_cmd_raw
        ) ~
        tag!(b"\r\n"),
        || { cmd }
    )
);

named!(parse_server_cmd_data(&[u8]) -> ServerCommand,
    chain!(
        tag!(b"DATA ") ~
        payload: many1!(hex_uint!(u8, 2)),
        || { ServerCommand::Data(payload) }
    )
);

named!(parse_server_cmd_error(&[u8]) -> ServerCommand,
    value!(ServerCommand::Error, tag!(b"ERROR"))
);

named!(parse_server_cmd_ok(&[u8]) -> ServerCommand,
    chain!(
        tag!(b"OK ") ~
        server_guid: parse_server_guid,
        || { ServerCommand::Ok { server_guid: server_guid } }
    )
);

named!(parse_server_cmd_rejected(&[u8]) -> ServerCommand,
    chain!(
        tag!(b"REJECTED ") ~
        supported_mechanisms: many0!(preceded!(tag!(b" "), parse_cmd_name)),
        || { ServerCommand::Rejected { supported_mechanisms: supported_mechanisms } }
    )
);

named!(parse_server_cmd_raw(&[u8]) -> ServerCommand,
    chain!(
        cmd: parse_cmd_name ~
        payload: opt!(preceded!(tag!(b" "), parse_cmd_name)),
        || { ServerCommand::Raw { cmd: cmd, payload: payload } }
    )
);

named!(parse_server_guid(&[u8]) -> ServerGuid,
    count_fixed!(u64, hex_uint!(u64, 16), 2)
);

named!(parse_cmd_name(&[u8]) -> Vec<u8>,
    map!(take_while1!(is_cmd_name_char), |xs: &[u8]| xs.to_vec())
);

fn is_cmd_name_char(c: u8) -> bool {
    (c >= b'A' && c <= b'Z') || c == b'_'
}

static HEX_CHARS: &'static [u8] = b"0123456789abcdef";

fn hex_encoded_len(src: &[u8]) -> usize {
    2 * src.len()
}

fn extend_from_hex_encoded(buf: &mut Vec<u8>, src: &[u8]) {
    for byte in src {
        buf.push(HEX_CHARS[(byte >> 4) as usize]);
        buf.push(HEX_CHARS[(byte & 0xf) as usize]);
    }
}
