extern crate futures;
extern crate tokio_core;
extern crate tokio_dbus;

use futures::Future;
use std::io::Error;
use tokio_core::reactor::Core;
use tokio_dbus::{AuthMode, Bus};

#[test]
fn test() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let bus = Bus::connect("/var/run/dbus/system_bus_socket",
                           &handle,
                           AuthMode::External);
    let disconnect = bus.map_err(Error::from).and_then(|bus| bus.disconnect());

    l.run(disconnect).unwrap();
}
