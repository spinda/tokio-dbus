use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{Error, ErrorKind};

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
