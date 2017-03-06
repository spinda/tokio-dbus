// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use futures::{future, Future, Sink, Stream};
use futures::future::Loop;
use libc;
use std::error;
use std::fmt::{self, Display, Formatter};
use std::io::{Error, ErrorKind};

use auth::client::Authenticator;
use auth::commands::{ClientCommand, ServerCommand, ServerGuid};

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

pub fn auth_external
    (auth: Authenticator)
     -> impl Future<Item = (ServerGuid, Authenticator), Error = (AuthError, Option<Authenticator>)> {
    let uid_str = unsafe { libc::getuid().to_string() };
    let initial_cmd = ClientCommand::Auth {
        mechanism: b"EXTERNAL"[..].into(),
        initial_response: Some(uid_str.into_bytes().into()),
    };
    future::loop_fn((auth, initial_cmd), |(auth, cmd)| {
        auth.send(cmd)
            .map_err(|err| (err.into(), None))
            .and_then(|auth| auth.into_future()
            .map_err(|(err, auth)| (err.into(), Some(auth))))
            .and_then(|(response, auth)| {
                match response {
                    Some(ServerCommand::Ok { server_guid }) => Ok(Loop::Break((server_guid, auth))),
                    Some(ServerCommand::Rejected { supported_mechanisms }) => {
                        Err((AuthError::Rejected { supported_mechanisms: supported_mechanisms },
                             Some(auth)))
                    }
                    Some(ServerCommand::Error) => Ok(Loop::Continue((auth, ClientCommand::Cancel))),
                    Some(_) => Ok(Loop::Continue((auth, ClientCommand::Error(None)))),
                    None => {
                        Err((Error::new(ErrorKind::UnexpectedEof,
                                        "unexpected EOF during authentication")
                                 .into(),
                             Some(auth)))
                    }
                }
            })
    })
}
