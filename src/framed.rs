use futures::{Async, Future, Poll};
use tokio_core::io::FramedIo;

pub fn read<T>(framed: T) -> Read<T>
    where T: FramedIo
{
    Read { framed: Some(framed) }
}

pub struct Read<T> {
    framed: Option<T>,
}

impl<T> Future for Read<T>
    where T: FramedIo
{
    type Item = (T, T::Out);
    type Error = ::std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let msg = match try!(self.framed.as_mut().unwrap().read()) {
            Async::Ready(msg) => msg,
            Async::NotReady => return Ok(Async::NotReady),
        };

        Ok(Async::Ready((self.framed.take().unwrap(), msg)))
    }
}

pub fn write<T>(framed: T, msg: T::In) -> Write<T>
    where T: FramedIo
{
    Write {
        framed: Some(framed),
        msg: msg,
    }
}

pub struct Write<T>
    where T: FramedIo
{
    framed: Option<T>,
    msg: T::In,
}

impl<T> Future for Write<T>
    where T: FramedIo
{
    type Item = T;
    type Error = ::std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::NotReady = try!(self.framed.as_mut().unwrap().write(&self.msg)) {
            return Ok(Async::NotReady);
        }

        Ok(Async::Ready(self.framed.take().unwrap()))
    }
}

pub fn flush<T>(framed: T) -> Flush<T>
    where T: FramedIo
{
    Flush { framed: Some(framed) }
}

pub struct Flush<T> {
    framed: Option<T>,
}

impl<T> Future for Flush<T>
    where T: FramedIo
{
    type Item = T;
    type Error = ::std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::NotReady = try!(self.framed.as_mut().unwrap().flush()) {
            return Ok(Async::NotReady);
        }

        Ok(Async::Ready(self.framed.take().unwrap()))
    }
}
