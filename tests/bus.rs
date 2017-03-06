extern crate tokio_dbus;

use tokio_dbus::{BasicType, ContainerType, Type};

#[test]
fn test() {
    let header_sig_enc = b"yyyyuua(yv)";
    let header_sig_ast = vec!(Type::BasicType(BasicType::Byte),
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
                                  Type::ContainerType(Box::new(ContainerType::Variant))))))))));

    let mut buf: Vec<u8> = vec![];
    tokio_dbus::encode_signature(&header_sig_ast, &mut buf);
    assert_eq!(buf.as_slice(), header_sig_enc);

    assert_eq!(tokio_dbus::decode_signature(b"yyyyuua(yv)").ok(),
               Some(Some((header_sig_ast, &b""[..]))));
}
