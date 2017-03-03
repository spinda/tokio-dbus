use futures::{Future, IntoFuture, Poll, Sink, StartSend, Stream};
use std::io::{Error, Result};
use std::net::Shutdown;
use std::path::Path;
use tokio_core::io::{self, Framed, Io};
use tokio_core::reactor::Handle;
use tokio_uds::UnixStream;

use bus::Bus;

use auth::codec::AuthCodec;
use auth::types::{ClientCommand, ServerCommand};

type AuthFramed = Framed<UnixStream, AuthCodec>;

pub struct Authenticator {
    inner: AuthFramed,
}

impl Authenticator {
    pub fn connect<P>(path: P, handle: &Handle) -> impl Future<Item = Self, Error = Error>
        where P: AsRef<Path>
    {
        UnixStream::connect(path, handle)
            .into_future()
            .map(Self::new)
            .and_then(Self::prime)
    }

    pub fn new(inner: UnixStream) -> Self {
        Authenticator { inner: inner.framed(AuthCodec) }
    }

    pub fn into_inner(self) -> UnixStream {
        self.inner.into_inner()
    }

    pub fn into_bus(self) -> Bus {
        Bus::new(self.into_inner())
    }

    pub fn prime(self) -> impl Future<Item = Self, Error = Error> {
        io::write_all(self.into_inner(), [0])
            .map(|(inner, _)| Authenticator::new(inner))
    }

    pub fn begin(self) -> impl Future<Item = Bus, Error = Error> {
        self.send(ClientCommand::Begin)
            .map(Authenticator::into_bus)
    }

    pub fn disconnect(self) -> Result<()> {
        self.into_inner().shutdown(Shutdown::Both)
    }
}

impl Stream for Authenticator {
    type Item = ServerCommand;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.poll()
    }
}

impl Sink for Authenticator {
    type SinkItem = ClientCommand;
    type SinkError = Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Error> {
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Error> {
        self.inner.poll_complete()
    }
}
