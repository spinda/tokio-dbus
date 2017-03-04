use futures::Future;
use std::io::Result;
use std::net::Shutdown;
use std::path::Path;
use tokio_core::reactor::Handle;
use tokio_uds::UnixStream;

use auth::{Authenticator, AuthError, ServerGuid};

pub struct Bus {
    inner: UnixStream,
}

impl Bus {
    pub fn connect<P, F, T>
        (path: P,
         handle: &Handle,
         auth_strategy: F)
         -> impl Future<Item = (ServerGuid, Self), Error = (AuthError, Option<Authenticator>)>
        where P: AsRef<Path>,
              F: FnOnce(Authenticator) -> T,
              T: Future<Item = (ServerGuid, Authenticator),
                        Error = (AuthError, Option<Authenticator>)>
    {
        Authenticator::connect(path, handle)
            .map_err(|err| (err.into(), None))
            .and_then(|auth| auth_strategy(auth))
            .and_then(|(server_guid, auth)| {
                auth.begin()
                    .map(move |bus| (server_guid, bus))
                    .map_err(|err| (err.into(), None))
            })
    }

    pub fn new(inner: UnixStream) -> Self {
        Bus { inner: inner }
    }

    pub fn into_inner(self) -> UnixStream {
        self.inner
    }

    pub fn disconnect(self) -> Result<()> {
        self.into_inner().shutdown(Shutdown::Both)
    }
}
