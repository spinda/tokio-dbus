// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.

pub type Signature = Vec<Type>;

#[derive(Eq, Debug, Clone, PartialEq)]
pub enum Type {
    BasicType(BasicType),
    ContainerType(Box<ContainerType>),
}

#[derive(Eq, Debug, Clone, PartialEq)]
pub enum BasicType {
    Byte,
    Boolean,
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

#[derive(Eq, Debug, Clone, PartialEq)]
pub enum ContainerType {
    Array(Type),
    Dict(BasicType, Type),
    Struct(Vec<Type>),
    Variant
}

named!(pub parse_signature(&[u8]) -> Signature,
    many1!(parse_type)
);

named!(pub parse_type(&[u8]) -> Type,
    alt!(
        map!(parse_basic_type, Type::BasicType) |
        do_parse!(ty: parse_container_type >> (Type::ContainerType(Box::new(ty))))
    )
);

named!(pub parse_basic_type(&[u8]) -> BasicType,
    alt!(
        do_parse!(tag!(b"y") >> (BasicType::Byte)) |
        do_parse!(tag!(b"b") >> (BasicType::Boolean)) |
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

named!(pub parse_container_type(&[u8]) -> ContainerType,
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
