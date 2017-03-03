use futures::{future, Future, Sink, Stream};
use futures::future::Loop;
use libc;
use std::io::{Error, ErrorKind};

use auth::client::Authenticator;
use auth::types::{AuthError, ClientCommand, ServerCommand, ServerGuid};

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
            .and_then(|auth| auth.into_future().map_err(|(err, auth)| (err.into(), Some(auth))))
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
