// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use nom::{self, IResult, Needed};
use std::borrow::Cow;
use std::io::{Error, ErrorKind, Result};

pub type ServerGuid = [u64; 2];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerCommand {
    Data(Vec<u8>),
    Error,
    Ok { server_guid: ServerGuid },
    Rejected { supported_mechanisms: Vec<Vec<u8>> },
    Raw {
        cmd: Vec<u8>,
        payload: Option<Vec<u8>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientCommand {
    Auth {
        mechanism: Cow<'static, [u8]>,
        initial_response: Option<Cow<'static, [u8]>>,
    },
    Begin,
    Cancel,
    Data(Cow<'static, [u8]>),
    Error(Option<Cow<'static, [u8]>>),
    Raw {
        cmd: Cow<'static, [u8]>,
        payload: Option<Cow<'static, [u8]>>,
    },
}

pub fn decode_server_cmd(input: &[u8]) -> Result<Option<(ServerCommand, &[u8])>> {
    match parse_server_cmd(input) {
        IResult::Done(remaining, cmd) => Ok(Some((cmd, remaining))),
        IResult::Incomplete(_) => Ok(None),
        IResult::Error(_) => {
            Err(Error::new(ErrorKind::InvalidData, "malformed D-Bus auth server command"))
        }
    }
}

pub fn encode_client_cmd(cmd: &ClientCommand, output: &mut Vec<u8>) {
    match *cmd {
        ClientCommand::Auth { ref mechanism, ref initial_response } => {
            match *initial_response {
                None => {
                    output.reserve_exact(5 + mechanism.len() + 2);

                    output.extend_from_slice(b"AUTH ");
                    // ^ 5 bytes
                    output.extend_from_slice(mechanism);
                    // ^ mechanism.len() bytes
                    output.extend_from_slice(b"\r\n");
                    // ^ 2 bytes
                }
                Some(ref initial_response) => {
                    output.reserve_exact(5 + mechanism.len() + 1 +
                                         hex_encoded_len(initial_response) +
                                         2);

                    output.extend_from_slice(b"AUTH ");
                    // ^ 5 bytes
                    output.extend_from_slice(mechanism);
                    // ^ mechanism.len() bytes
                    output.push(b' ');
                    // ^ 1 byte
                    extend_from_hex_encoded(output, initial_response);
                    // ^ hex_encoded_len(&initial_response) bytes
                    output.extend_from_slice(b"\r\n");
                    // ^ 2 bytes
                }
            }
        }
        ClientCommand::Begin => output.extend_from_slice(b"BEGIN\r\n"),
        ClientCommand::Cancel => output.extend_from_slice(b"CANCEL\r\n"),
        ClientCommand::Data(ref bytes) => {
            output.reserve_exact(5 + hex_encoded_len(&bytes) + 2);

            output.extend_from_slice(b"DATA ");
            // ^ 5 bytes
            extend_from_hex_encoded(output, &bytes);
            // ^ hex_encoded_len(&bytes) bytes
            output.extend_from_slice(b"\r\n");
            // ^ 2 bytes
        }
        ClientCommand::Error(ref message) => {
            match *message {
                None => output.extend_from_slice(b"ERROR\r\n"),
                Some(ref message) => {
                    output.reserve_exact(6 + message.len() + 2);

                    output.extend_from_slice(b"ERROR ");
                    // ^ 6 bytes
                    output.extend_from_slice(message);
                    // ^ message.len() bytes
                    output.extend_from_slice(b"\r\n");
                    // ^ 2 bytes
                }
            }
        }
        ClientCommand::Raw { ref cmd, ref payload } => {
            match *payload {
                None => output.extend_from_slice(&cmd),
                Some(ref payload) => {
                    output.reserve_exact(cmd.len() + 1 + payload.len() + 2);

                    output.extend_from_slice(cmd);
                    // ^ cmd.len() bytes
                    output.push(b' ');
                    // ^ 1 byte
                    output.extend_from_slice(payload);
                    // ^ payload.len() bytes
                    output.extend_from_slice(b"\r\n");
                    // ^ 2 bytes
                }
            }
        }
    }
}

macro_rules! hex_uint(
    ($input:expr, $typ:ty, $n:expr) => (
        {
            if $input.len() < $n {
                IResult::Incomplete::<_, _>(Needed::Size($n))
            } else {
                let mut acc: $typ = 0;
                for c in &$input[0..$n] {
                    acc <<= 4;
                    match *c {
                        c @ b'0' ... b'9' => acc += (c - b'0') as $typ,
                        c @ b'a' ... b'f' => acc += (c - b'a' + 10) as $typ,
                        _ => return IResult::Error(nom::ErrorKind::HexDigit)
                    }
                }
                IResult::Done(&$input[$n..], acc)
            }
        }
    )
);

named!(parse_server_cmd(&[u8]) -> ServerCommand,
    do_parse!(
        cmd: alt!(
            parse_server_cmd_data |
            parse_server_cmd_error |
            parse_server_cmd_ok |
            parse_server_cmd_rejected |
            parse_server_cmd_raw
        ) >>
        tag!(b"\r\n") >>
        (cmd)
    )
);

named!(parse_server_cmd_data(&[u8]) -> ServerCommand,
    do_parse!(
        tag!(b"DATA ") >>
        payload: many1!(hex_uint!(u8, 2)) >>
        (ServerCommand::Data(payload))
    )
);

named!(parse_server_cmd_error(&[u8]) -> ServerCommand,
    value!(ServerCommand::Error, tag!(b"ERROR"))
);

named!(parse_server_cmd_ok(&[u8]) -> ServerCommand,
    do_parse!(
        tag!(b"OK ") >>
        server_guid: parse_server_guid >>
        (ServerCommand::Ok { server_guid: server_guid })
    )
);

named!(parse_server_cmd_rejected(&[u8]) -> ServerCommand,
    do_parse!(
        tag!(b"REJECTED ") >>
        supported_mechanisms: many0!(preceded!(tag!(b" "), parse_cmd_name)) >>
        (ServerCommand::Rejected { supported_mechanisms: supported_mechanisms })
    )
);

named!(parse_server_cmd_raw(&[u8]) -> ServerCommand,
    do_parse!(
        cmd: parse_cmd_name >>
        payload: opt!(preceded!(tag!(b" "), parse_cmd_name)) >>
        (ServerCommand::Raw { cmd: cmd, payload: payload })
    )
);

named!(parse_server_guid(&[u8]) -> ServerGuid,
    count_fixed!(u64, hex_uint!(u64, 16), 2)
);

named!(parse_cmd_name(&[u8]) -> Vec<u8>,
    map!(take_while1!(is_cmd_name_char), |xs: &[u8]| xs.to_vec())
);

fn is_cmd_name_char(c: u8) -> bool {
    (c >= b'A' && c <= b'Z') || c == b'_'
}

static HEX_CHARS: &'static [u8] = b"0123456789abcdef";

fn hex_encoded_len(src: &[u8]) -> usize {
    2 * src.len()
}

fn extend_from_hex_encoded(buf: &mut Vec<u8>, src: &[u8]) {
    for byte in src {
        buf.push(HEX_CHARS[(byte >> 4) as usize]);
        buf.push(HEX_CHARS[(byte & 0xf) as usize]);
    }
}
