use crate::lib::{
    fmt::{self, Display},
    iter::Iterator,
    String, Vec,
};
use core::ops::Deref;
use serde::{Deserialize, Serialize};

/// A collection of Addresses
#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct AddressSet(Vec<Address>);

impl AddressSet {
    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.0.iter()
    }

    pub fn first(&self) -> Address {
        self.0.first().cloned().unwrap()
    }
}

impl<T: Into<Address>> From<Vec<T>> for AddressSet {
    fn from(v: Vec<T>) -> Self {
        Self(v.into_iter().map(Into::into).collect())
    }
}

impl From<Address> for AddressSet {
    fn from(a: Address) -> Self {
        Self(vec![a])
    }
}

impl<'a> From<&'a Address> for AddressSet {
    fn from(a: &'a Address) -> Self {
        Self(vec![a.clone()])
    }
}

impl<'a> From<&'a str> for AddressSet {
    fn from(a: &'a str) -> Self {
        Self(vec![a.into()])
    }
}

/// An external identifier for message routing.
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
