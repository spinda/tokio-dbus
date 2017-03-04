#![feature(conservative_impl_trait)]

extern crate futures;
extern crate libc;
#[macro_use]
extern crate nom;
extern crate tokio_core;
extern crate tokio_uds;

pub mod auth;
pub mod bus;

pub use auth::{Authenticator, AuthError, ClientCommand, ServerCommand, ServerGuid, auth_external};
pub use bus::{Bus, Signature, Type, BasicType, ContainerType, parse_signature, parse_type,
              parse_basic_type, parse_container_type};
