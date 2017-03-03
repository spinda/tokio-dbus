extern crate futures;
extern crate tokio_core;
extern crate tokio_dbus;

use futures::Future;
use tokio_core::reactor::Core;
use tokio_dbus::Bus;

#[test]
fn test() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    l.run(Bus::connect("/var/run/dbus/system_bus_socket",
                          &handle,
                          tokio_dbus::auth_external)
            .map_err(|(err, _)| err)
            .and_then(|(_, bus)| bus.disconnect().map_err(Into::into)))
        .unwrap()
}
