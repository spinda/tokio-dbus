extern crate nom;
extern crate tokio_dbus;

use nom::IResult;
use tokio_dbus::{BasicType, ContainerType, Type};

#[test]
fn test() {
    let expected = IResult::Done(&b""[..],
        vec!(Type::BasicType(BasicType::Byte),
             Type::BasicType(BasicType::Byte),
             Type::BasicType(BasicType::Byte),
             Type::BasicType(BasicType::Byte),
             Type::BasicType(BasicType::UInt32),
             Type::BasicType(BasicType::UInt32),
             Type::ContainerType(Box::new(
                 ContainerType::Array(
                     Type::ContainerType(Box::new(
                         ContainerType::Struct(
                             vec!(Type::BasicType(BasicType::Byte),
                                  Type::ContainerType(Box::new(ContainerType::Variant)))))))))));
    assert_eq!(tokio_dbus::parse_signature(b"yyyyuua(yv)"), expected);
}
