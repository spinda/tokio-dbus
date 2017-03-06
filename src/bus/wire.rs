// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;
use std::os::unix::io::RawFd;

use bus::types::Signature;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    BasicValue(BasicValue),
    ContainerValue(ContainerValue),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BasicValue {
    Byte(u8),
    Bool(bool),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Double(f64),
    String(Cow<'static, str>),
    ObjectPath(Cow<'static, [u8]>),
    Signature(Signature),
    UnixFd(RawFd),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContainerValue {
    Array(Vec<Value>),
    Struct(Vec<Value>),
    Variant(Box<Value>),
    Dict(Vec<(BasicValue, Value)>),
}
