use crate::lib::{
    fmt::{self, Debug, Display},
    str::from_utf8,
    String, ToString, Vec,
};
use core::ops::Deref;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// A collection of Addresses
#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct AddressSet(Vec<Address>);

impl AddressSet {
    /// Retrieve the set's iterator.
    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.0.iter()
    }

    /// Turn this set into an iterator
    pub fn into_iter(self) -> impl Iterator<Item = Address> {
        self.0.into_iter()
    }

    /// Take the first address of the set.
    pub fn first(&self) -> Address {
        self.0.first().cloned().unwrap()
    }
}

impl AsRef<Vec<Address>> for AddressSet {
    fn as_ref(&self) -> &Vec<Address> {
        &self.0
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

impl Address {
    /// Parse an address from a string
    ///
    /// See type documentation for more detail
    pub fn from_string<S: Into<String>>(s: S) -> Self {
        let buf: String = s.into();
        let mut vec: Vec<_> = buf.split("#").collect();

        // If after the split we only have one element, there was no
        // `#` separator, so the type needs to be implicitly `= 0`
        let (tt, inner) = if vec.len() == 1 {
            (0, vec.remove(0).as_bytes().to_vec())
        }
        // If after the split we have 2 elements, we extract the type
        // value from the string, and use the rest as the address
        else if vec.len() == 2 {
            let tt = match str::parse(vec.remove(0)) {
                Ok(tt) => tt,
                Err(e) => {
                    panic!("Failed to parse address type: '{}'", e);
                }
            };

            (tt, vec.remove(0).as_bytes().to_vec())
        } else {
            panic!("Invalid address string: more than one `#` separator found");
        };

        Self { tt, inner }
    }

    /// Generate a random address with a specific type
    pub fn random(tt: u8) -> Self {
        let mut rng = rand::thread_rng();
        let address: [u8; 16] = rng.gen();
        let inner = hex::encode(address).as_bytes().into();
        Self { tt, inner }
    }
}

impl Display for Address {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner: &'a str = from_utf8(&self.inner.as_slice()).unwrap_or("Invalid UTF-8");
        write!(f, "{}#{}", self.tt, inner)
    }
}

impl Debug for Address {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner: &'a str = from_utf8(&self.inner.as_slice()).unwrap_or("Invalid UTF-8");
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
#[should_panic]
fn parse_addr_invalid() {
    let _ = Address::from_string("1#invalid#");
}
