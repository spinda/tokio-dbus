// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

mod client;
mod commands;
pub mod strategies;

pub use auth::client::Authenticator;
pub use auth::commands::{ClientCommand, ServerCommand, ServerGuid, decode_server_cmd,
                         encode_client_cmd};
pub use auth::strategies::{AuthError, auth_external};
