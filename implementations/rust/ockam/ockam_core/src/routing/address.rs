use crate::compat::rand::{distributions::Standard, prelude::Distribution, random, Rng};
use crate::compat::{
    string::{String, ToString},
    vec::{self, Vec},
};
use core::fmt::{self, Debug, Display};
use core::iter::FromIterator;
use core::ops::Deref;
use core::str::from_utf8;
use serde::{Deserialize, Serialize};

/// A collection of Addresses
#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct AddressSet(Vec<Address>);

impl AddressSet {
    /// Retrieve the set's iterator.
    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.0.iter()
    }

    /// Take the first address of the set.
    pub fn first(&self) -> Address {
        self.0.first().cloned().unwrap()
    }

    /// Check if an address is contained in this set
    pub fn contains(&self, a2: &Address) -> bool {
        self.0
            .iter()
            .find(|a1| a1 == &a2)
            .map(|_| true)
            .unwrap_or(false)
    }
}

impl IntoIterator for AddressSet {
    type Item = Address;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl AsRef<[Address]> for AddressSet {
    fn as_ref(&self) -> &[Address] {
        &self.0
    }
}

impl<A> FromIterator<A> for AddressSet
where
    A: Into<Address>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = A>,
    {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Address>> From<Vec<T>> for AddressSet {
    fn from(v: Vec<T>) -> Self {
        v.into_iter().collect()
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

/// A generic component address
///
/// The address type is parsed by routers to determine the next local
/// hop in the router chain to resolve a route to a remote connection.
///
/// ## Parsing addresses
///
/// While addresses are concrete types, creating them from strings is
/// possible for ergonomics reasons.  When parsing an address from a
/// string, the first `#` symbol is used to separate the type from the
/// rest of the address.  If no `#` symbol is found, the address is
/// assumed to be of `tt = 0` (local worker).
#[derive(Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Address {
    /// The address type
    pub tt: u8,
    inner: Vec<u8>,
}
/// An error which is returned when address parsing from string fails
#[derive(Debug)]
pub struct AddressParseError {
    kind: AddressParseErrorKind,
}
/// Enum to store the cause of address parsing failure
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AddressParseErrorKind {
    /// Unable to parse address num in the address string
    InvalidType(core::num::ParseIntError),
    /// Address string has more than one '#' separator
    MultipleSep,
}

impl AddressParseError {
    /// Create new instance
    pub fn new(kind: AddressParseErrorKind) -> Self {
        Self { kind }
    }
    /// Gets the cause of address parsing failure.
    pub fn kind(&self) -> &AddressParseErrorKind {
        &self.kind
    }
}
impl Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            AddressParseErrorKind::InvalidType(e) => {
                write!(f, "Failed to parse address type: '{}'", e)
            }
            AddressParseErrorKind::MultipleSep => {
                write!(
                    f,
                    "Invalid address string: more than one '#' separator found"
                )
            }
        }
    }
}
impl crate::compat::error::Error for AddressParseError {}
impl Address {
    /// Create a new address from separate type and data parts
    pub fn new<S: Into<String>>(tt: u8, inner: S) -> Self {
        Self {
            tt,
            inner: inner.into().as_bytes().to_vec(),
        }
    }

    /// Parse an address from a string
    ///
    /// See type documentation for more detail
    pub fn from_string<S: Into<String>>(s: S) -> Self {
        match s.into().parse::<Address>() {
            Ok(a) => a,
            Err(e) => {
                panic!("Invalid address string {}", e)
            }
        }
    }
    /// Generate a random address with a specific type
    pub fn random(tt: u8) -> Self {
        Self { tt, ..random() }
    }
}
impl core::str::FromStr for Address {
    type Err = AddressParseError;
    /// Parse an address from a string
    ///
    /// See type documentation for more detail
    fn from_str(s: &str) -> Result<Address, Self::Err> {
        let buf: String = s.into();
        let mut vec: Vec<_> = buf.split('#').collect();

        // If after the split we only have one element, there was no
        // `#` separator, so the type needs to be implicitly `= 0`
        if vec.len() == 1 {
            Ok(Address {
                tt: 0,
                inner: vec.remove(0).as_bytes().to_vec(),
            })
        }
        // If after the split we have 2 elements, we extract the type
        // value from the string, and use the rest as the address
        else if vec.len() == 2 {
            match str::parse(vec.remove(0)) {
                Ok(tt) => Ok(Address {
                    tt,
                    inner: vec.remove(0).as_bytes().to_vec(),
                }),
                Err(e) => Err(AddressParseError::new(AddressParseErrorKind::InvalidType(
                    e,
                ))),
            }
        } else {
            Err(AddressParseError::new(AddressParseErrorKind::MultipleSep))
        }
    }
}
impl Display for Address {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner: &'a str = from_utf8(self.inner.as_slice()).unwrap_or("Invalid UTF-8");
        write!(f, "{}#{}", self.tt, inner)
    }
}

impl Debug for Address {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner: &'a str = from_utf8(self.inner.as_slice()).unwrap_or("Invalid UTF-8");
        write!(f, "{}#{}", self.tt, inner)
    }
}

impl Deref for Address {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl<'a> From<&'a str> for Address {
    fn from(s: &'a str) -> Self {
        Self::from_string(s)
    }
}

impl From<Vec<u8>> for Address {
    fn from(inner: Vec<u8>) -> Self {
        Self { tt: 0, inner }
    }
}

impl From<(u8, Vec<u8>)> for Address {
    fn from((tt, inner): (u8, Vec<u8>)) -> Self {
        Self { tt, inner }
    }
}

impl<'a> From<(u8, &'a str)> for Address {
    fn from((tt, inner): (u8, &'a str)) -> Self {
        Self {
            tt,
            inner: inner.as_bytes().to_vec(),
        }
    }
}

impl From<(u8, String)> for Address {
    fn from((tt, inner): (u8, String)) -> Self {
        Self::from((tt, inner.as_str()))
    }
}

impl<'a> From<&'a [u8]> for Address {
    fn from(inner: &'a [u8]) -> Self {
        Self {
            tt: 0,
            inner: inner.to_vec(),
        }
    }
}

impl<'a> From<&'a [&u8]> for Address {
    fn from(inner: &'a [&u8]) -> Self {
        Self {
            tt: 0,
            inner: inner.iter().map(|x| **x).collect(),
        }
    }
}

impl From<Address> for String {
    fn from(addr: Address) -> Self {
        addr.to_string()
    }
}

impl Distribution<Address> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Address {
        let address: [u8; 16] = rng.gen();
        hex::encode(address).as_bytes().into()
    }
}

#[test]
fn parse_addr_simple() {
    let addr = Address::from_string("local_friend");
    assert_eq!(
        addr,
        Address {
            tt: 0,
            inner: "local_friend".as_bytes().to_vec()
        }
    );
}

#[test]
fn parse_addr_with_type() {
    let addr = Address::from_string("1#remote_friend");
    assert_eq!(
        addr,
        Address {
            tt: 1,
            inner: "remote_friend".as_bytes().to_vec()
        }
    );
}

#[test]
#[should_panic(expected = "Failed to parse address type:")]
fn parse_addr_invalid() {
    Address::from_string("#,my_friend");
}

#[test]
#[should_panic(expected = "Invalid address string:")]
fn parse_addr_invalid_multiple_separators() {
    let _ = Address::from_string("1#invalid#");
}
