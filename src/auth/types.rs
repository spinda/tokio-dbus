use std::borrow::Cow;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{Error, ErrorKind};

pub type ServerGuid = [u64; 2];

#[derive(Debug)]
pub enum AuthError {
    Io(Error),
    Rejected { supported_mechanisms: Vec<Vec<u8>> },
}

impl Display for AuthError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            AuthError::Io(ref err) => err.fmt(f),
            AuthError::Rejected { .. } => {
                write!(f,
                       "Authentication attempt was rejected by the D-Bus server.")
            }
        }
    }
}

impl error::Error for AuthError {
    fn description(&self) -> &str {
        match *self {
            AuthError::Io(ref err) => err.description(),
            AuthError::Rejected { .. } => "D-Bus authentication rejected",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            AuthError::Io(ref err) => Some(err),
            AuthError::Rejected { .. } => None,
        }
    }
}

impl From<AuthError> for Error {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::Io(e) => e,
            AuthError::Rejected { .. } => {
                Error::new(ErrorKind::PermissionDenied, "D-Bus authentication rejected")
            }
        }
    }
}

impl From<Error> for AuthError {
    fn from(err: Error) -> Self {
        AuthError::Io(err)
    }
}

#[derive(Clone, Debug)]
pub enum ServerCommand {
    Data(Vec<u8>),
    Error,
    Ok { server_guid: ServerGuid },
    Rejected { supported_mechanisms: Vec<Vec<u8>> },
    Raw {
        cmd: Vec<u8>,
        payload: Option<Vec<u8>>,
    },
}

#[derive(Clone, Debug)]
pub enum ClientCommand {
    Auth {
        mechanism: Cow<'static, [u8]>,
        initial_response: Option<Cow<'static, [u8]>>,
    },
    Begin,
    Cancel,
    Data(Cow<'static, [u8]>),
    Error(Option<Cow<'static, [u8]>>),
    Raw {
        cmd: Cow<'static, [u8]>,
        payload: Option<Cow<'static, [u8]>>,
    },
}
