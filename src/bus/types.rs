// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use nom::{IResult};
use std::io::{Error, ErrorKind, Result};

pub type Signature = Vec<Type>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    BasicType(BasicType),
    ContainerType(Box<ContainerType>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BasicType {
    Byte,
    Bool,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Double,
    String,
    ObjectPath,
    Signature,
    UnixFd,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContainerType {
    Array(Type),
    Dict(BasicType, Type),
    Struct(Vec<Type>),
    Variant,
}

pub fn decode_signature(input: &[u8]) -> Result<Option<(Signature, &[u8])>> {
    match parse_signature(input) {
        IResult::Done(remaining, signature) => Ok(Some((signature, remaining))),
        IResult::Incomplete(_) => Ok(None),
        IResult::Error(_) => Err(Error::new(ErrorKind::InvalidData, "malformed D-Bus signature")),
    }
}

pub fn encode_signature(signature: &Signature, output: &mut Vec<u8>) {
    // The minimum type encoding length is 1 byte, so expect at least 1 byte
    // per type in the signature.
    output.reserve(signature.len());

    encode_types(signature, output);
}

fn encode_types(tys: &Vec<Type>, output: &mut Vec<u8>) {
    for ty in tys {
        encode_type(ty, output);
    }
}

fn encode_type(ty: &Type, output: &mut Vec<u8>) {
    match *ty {
        Type::BasicType(ref basic_ty) => encode_basic_type(basic_ty, output),
        Type::ContainerType(ref container_ty) => encode_container_type(container_ty, output),
    }
}

fn encode_basic_type(ty: &BasicType, output: &mut Vec<u8>) {
    match *ty {
        BasicType::Byte => output.push(b'y'),
        BasicType::Bool => output.push(b'b'),
        BasicType::Int16 => output.push(b'n'),
        BasicType::UInt16 => output.push(b'q'),
        BasicType::Int32 => output.push(b'i'),
        BasicType::UInt32 => output.push(b'u'),
        BasicType::Int64 => output.push(b'x'),
        BasicType::UInt64 => output.push(b't'),
        BasicType::Double => output.push(b'd'),
        BasicType::String => output.push(b's'),
        BasicType::ObjectPath => output.push(b'o'),
        BasicType::Signature => output.push(b'g'),
        BasicType::UnixFd => output.push(b'h'),
    }
}

fn encode_container_type(ty: &ContainerType, output: &mut Vec<u8>) {
    match *ty {
        ContainerType::Array(ref inner_ty) => {
            output.push(b'a');
            encode_type(inner_ty, output);
        }
        ContainerType::Dict(ref key_ty, ref value_ty) => {
            // We know we're going to push 1 byte for the opening brace, at
            // least 1 byte for each of the key and value types, and 1 byte for
            // the closing brace. We'll already have reserved a byte for the 'a'
            // previously in the call chain.
            output.reserve(4);

            output.extend_from_slice(b"a{");
            encode_basic_type(key_ty, output);
            encode_type(value_ty, output);
            output.push(b'}');
        }
        ContainerType::Struct(ref inner_tys) => {
            // We know we're going to push at least 1 byte for each of the inner
            // types and 1 byte for the closing paren. We'll already have
            // reserved a byte for the opening paren previously in the call
            // chain.
            output.reserve(inner_tys.len() + 1);

            output.push(b'(');
            encode_types(inner_tys, output);
            output.push(b')');
        }
        ContainerType::Variant => output.push(b'v'),
    }
}

named!(parse_signature(&[u8]) -> Signature,
    many1!(parse_type)
);

named!(parse_type(&[u8]) -> Type,
    alt!(
        map!(parse_basic_type, Type::BasicType) |
        do_parse!(ty: parse_container_type >> (Type::ContainerType(Box::new(ty))))
    )
);

named!(parse_basic_type(&[u8]) -> BasicType,
    alt!(
        do_parse!(tag!(b"y") >> (BasicType::Byte)) |
        do_parse!(tag!(b"b") >> (BasicType::Bool)) |
        do_parse!(tag!(b"n") >> (BasicType::Int16)) |
        do_parse!(tag!(b"q") >> (BasicType::UInt16)) |
        do_parse!(tag!(b"i") >> (BasicType::Int32)) |
        do_parse!(tag!(b"u") >> (BasicType::UInt32)) |
        do_parse!(tag!(b"x") >> (BasicType::Int64)) |
        do_parse!(tag!(b"t") >> (BasicType::UInt64)) |
        do_parse!(tag!(b"d") >> (BasicType::Double)) |
        do_parse!(tag!(b"s") >> (BasicType::String)) |
        do_parse!(tag!(b"o") >> (BasicType::ObjectPath)) |
        do_parse!(tag!(b"g") >> (BasicType::Signature)) |
        do_parse!(tag!(b"h") >> (BasicType::UnixFd))
    )
);

named!(parse_container_type(&[u8]) -> ContainerType,
    alt!(
        parse_array_or_dict_type |
        parse_struct_type |
        do_parse!(tag!(b"v") >> (ContainerType::Variant))
    )
);

named!(parse_array_or_dict_type(&[u8]) -> ContainerType,
    preceded!(
        tag!(b"a"),
        alt!(
            do_parse!(
                tag!(b"{") >>
                key: parse_basic_type >>
                val: parse_type >>
                tag!(b"}") >>
                (ContainerType::Dict(key, val))
            ) |
            map!(parse_type, ContainerType::Array)
        )
    )
);

named!(parse_struct_type(&[u8]) -> ContainerType,
    do_parse!(
        tag!(b"(") >>
        inner: many1!(parse_type) >>
        tag!(b")") >>
        (ContainerType::Struct(inner))
    )
);
