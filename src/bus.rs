use futures::{self, Future};
use std::io::Result;
use std::net::Shutdown;
use std::path::Path;
use tokio_core::reactor::Handle;
use tokio_uds::UnixStream;

use auth;
pub use auth::{AuthError, AuthFuture, AuthMode};

pub struct Bus {
    upstream: UnixStream,
}

impl Bus {
    pub fn connect<P>(path: P, handle: &Handle, auth_mode: AuthMode) -> AuthFuture<Self>
        where P: AsRef<Path>
    {
        let upstream = futures::done(UnixStream::connect(path, handle))
            .map_err(AuthError::from);
        let bus =
            upstream.and_then(|upstream| Bus::from_upstream(upstream).authenticate(auth_mode));

        bus.boxed()
    }

    pub fn from_upstream(upstream: UnixStream) -> Self {
        Bus { upstream: upstream }
    }

    pub fn into_upstream(self) -> UnixStream {
        self.upstream
    }

    pub fn authenticate(self, auth_mode: AuthMode) -> AuthFuture<Self> {
        let Bus { upstream } = self;

        let auth = auth::authenticate(upstream, auth_mode);
        let pack = auth.map(Bus::from_upstream);

        pack.boxed()
    }

    pub fn disconnect(&self) -> Result<()> {
        self.upstream.shutdown(Shutdown::Both)
    }
}
