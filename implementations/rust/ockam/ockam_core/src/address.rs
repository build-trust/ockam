use crate::lib::{
    fmt::{self, Display},
    String, Vec,
};
use core::ops::Deref;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct Address(Vec<u8>);

impl Display for Address {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        let s: &'a str = self.into();
        write!(f, "{}", s)
    }
}

impl Deref for Address {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl<'a> From<&'a str> for Address {
    fn from(s: &'a str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl From<Vec<u8>> for Address {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl<'a> From<&'a [u8]> for Address {
    fn from(v: &'a [u8]) -> Self {
        Self(v.to_vec())
    }
}

impl<'a> From<&'a [&u8]> for Address {
    fn from(v: &'a [&u8]) -> Self {
        Self(v.iter().map(|x| **x).collect())
    }
}

impl From<Address> for String {
    fn from(addr: Address) -> Self {
        String::from_utf8(addr.0).unwrap()
    }
}

impl<'a> From<&'a Address> for &'a str {
    fn from(addr: &'a Address) -> Self {
        core::str::from_utf8(addr.0.as_slice()).unwrap()
    }
}
