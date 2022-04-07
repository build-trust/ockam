use crate::compat::rand;
use crate::compat::{
    string::{String, ToString},
    vec::{self, Vec},
};
use crate::error::errcode::{Kind, Origin};
use core::fmt::{self, Debug, Display};
use core::iter::FromIterator;
use serde::{Deserialize, Serialize};

/// A read-only sequence of [`Address`]es.
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

    /// Check if an address is contained in this set.
    pub fn contains(&self, a2: &Address) -> bool {
        self.0.contains(a2)
    }

    /// Try to create an address set from an iterator over [`Address`]-like types.
    pub fn try_from_iter<A, I>(iter: I) -> Result<Self, A::Error>
    where
        A: TryInto<Address>,
        I: IntoIterator<Item = A>,
    {
        let mut v = Vec::new();
        for a in iter.into_iter() {
            v.push(a.try_into()?)
        }
        Ok(AddressSet(v))
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

impl From<&Address> for AddressSet {
    fn from(a: &Address) -> Self {
        Self(vec![a.clone()])
    }
}

impl From<(TransportType, &str)> for AddressSet {
    fn from((t, a): (TransportType, &str)) -> Self {
        Self(vec![(t, a).into()])
    }
}

impl TryFrom<&str> for AddressSet {
    type Error = AddressParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Address::try_from(value).map(AddressSet::from)
    }
}

impl TryFrom<String> for AddressSet {
    type Error = AddressParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Address::try_from(value).map(AddressSet::from)
    }
}

/// A generic address type.
///
/// The address type is parsed by routers to determine the next local
/// hop in the router chain to resolve a route to a remote connection.
///
/// When parsed from strings, addresses are of the following format:
///
/// ```text
///     Address <- Full / Local
///     Full    <- [0-9]+ '#' Char*
///     Local   <- !'#' Char*
/// ```
#[derive(Serialize, Deserialize, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Address {
    tt: TransportType,
    addr: Vec<u8>,
}

impl Address {
    /// Creates a new address from separate transport type and data parts.
    pub fn new<S: Into<String>>(tt: TransportType, data: S) -> Self {
        Address {
            tt,
            addr: data.into().into_bytes(),
        }
    }

    /// Create a new address with transport type [`LOCAL`].
    pub fn local<T: Into<String>>(data: T) -> Self {
        Address::new(LOCAL, data.into())
    }

    /// Generate a random address with the given transport type.
    pub fn random(tt: TransportType) -> Self {
        Address::new(tt, hex::encode(rand::random::<[u8; 16]>()))
    }

    /// Generate a random address with transport type [`LOCAL`].
    pub fn random_local() -> Self {
        Address::random(LOCAL)
    }

    /// Get transport type of this address.
    pub fn transport_type(&self) -> TransportType {
        self.tt
    }

    /// Access address data without transport type.
    pub fn data(&self) -> &[u8] {
        &self.addr
    }
}

impl core::str::FromStr for Address {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Address, Self::Err> {
        match s.split_once('#') {
            Some((pre, suf)) => {
                let n = str::parse(pre).map_err(AddressParseError)?;
                Ok(Address::new(TransportType::new(n), suf.to_string()))
            }
            None => Ok(Address::local(s.to_string())),
        }
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(s) = core::str::from_utf8(self.data()) {
            write!(f, "{}#{}", self.transport_type(), s)
        } else {
            write!(f, "{}#{}", self.transport_type(), hex::encode(self.data()))
        }
    }
}

impl TryFrom<&str> for Address {
    type Error = AddressParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        core::str::FromStr::from_str(value)
    }
}

impl TryFrom<String> for Address {
    type Error = AddressParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        core::str::FromStr::from_str(value.as_str())
    }
}

impl From<(TransportType, &str)> for Address {
    fn from((tt, data): (TransportType, &str)) -> Self {
        Address::new(tt, data)
    }
}

impl From<(TransportType, String)> for Address {
    fn from((tt, data): (TransportType, String)) -> Self {
        Address::new(tt, data)
    }
}

impl From<(TransportType, &String)> for Address {
    fn from((tt, data): (TransportType, &String)) -> Self {
        Address::new(tt, data)
    }
}

/// The transport type of an address.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct TransportType(u8);

/// The local transport type.
pub const LOCAL: TransportType = TransportType::new(0);

impl TransportType {
    /// Create a new transport type.
    pub const fn new(n: u8) -> Self {
        TransportType(n)
    }

    /// Is this the local transport type?
    pub fn is_local(self) -> bool {
        self == LOCAL
    }
}

impl Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TransportType> for u8 {
    fn from(ty: TransportType) -> Self {
        ty.0
    }
}

/// An error which is returned when address parsing from string fails.
#[derive(Debug)]
pub struct AddressParseError(core::num::ParseIntError);

impl Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Failed to parse address type.")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AddressParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[cfg(not(feature = "std"))]
impl crate::compat::error::Error for AddressParseError {}

impl From<AddressParseError> for crate::Error {
    fn from(e: AddressParseError) -> Self {
        crate::Error::new(Origin::Unknown, Kind::Invalid, e)
    }
}

#[cfg(test)]
mod tests {
    use super::{Address, TransportType, LOCAL};

    #[test]
    fn parse_addr_simple() {
        let addr = Address::try_from("local_friend").unwrap();
        assert_eq!(addr, Address::new(LOCAL, "local_friend"));
    }

    #[test]
    fn parse_addr_with_type() {
        let addr = Address::try_from("1#remote_friend").unwrap();
        assert_eq!(addr, Address::new(TransportType::new(1), "remote_friend"))
    }

    #[test]
    #[should_panic]
    fn parse_addr_invalid() {
        Address::try_from("#,my_friend").unwrap();
    }
}
