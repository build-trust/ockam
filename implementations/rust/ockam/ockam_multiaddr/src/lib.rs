//! An implementation of multiformats.io/multiaddr.
//!
//! The main entities of this crate are:
//!
//! - [`MultiAddr`]: A sequence of protocol values.
//! - [`Protocol`]: A type that can be read from and written to strings and bytes.
//! - [`Codec`]: A type that understands protocols.
//! - [`ProtoValue`]: A section of a MultiAddr.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod error;
mod registry;

pub mod codec;
pub mod iter;
pub mod proto;

use core::fmt;
use core::ops::Deref;
use once_cell::sync::Lazy;
use tinyvec::{Array, ArrayVec, TinyVec};

pub use error::Error;
pub use registry::{Registry, RegistryBuilder};

/// Global default registry of known protocols.
static DEFAULT_REGISTRY: Lazy<Registry> = Lazy::new(Registry::default);

/// Component of a [`MultiAddr`].
///
/// A protocol supports a textual and a binary representation.
///
/// ```text
/// Protocol <- Text / Binary
/// Text     <- '/' Prefix '/' Char+
/// Prefix   <- Char+
/// Binary   <- Code Byte+
/// Code     <- UnsignedVarint
/// ```
///
/// To process a protocol, one needs to know the code and prefix as they
/// determine the protocol value.
///
/// NB: Protocol values which contain '/'s create ambiguity in the textual
/// representation. These so called "path protocols" must be the last
/// protocol component in a multi-address.
pub trait Protocol<'a>: Sized {
    /// Registered protocol code.
    const CODE: Code;
    /// Registered protocol prefix.
    const PREFIX: &'static str;

    /// Parse the string value of this protocol.
    fn read_str(input: Checked<&'a str>) -> Result<Self, Error>;

    /// Write the protocol as a string, including the prefix.
    fn write_str(&self, f: &mut fmt::Formatter) -> Result<(), Error>;

    /// Decode the binary value of this protocol.
    fn read_bytes(input: Checked<&'a [u8]>) -> Result<Self, Error>;

    /// Write the protocol as a binary value, including the code.
    fn write_bytes(&self, buf: &mut dyn Buffer);
}

/// Type that understands how to read and write [`Protocol`]s.
pub trait Codec: Send + Sync {
    /// Split input string into the value and the remainder.
    fn split_str<'a>(
        &self,
        prefix: &str,
        input: &'a str,
    ) -> Result<(Checked<&'a str>, &'a str), Error>;

    /// Split input bytes into the value and the remainder.
    fn split_bytes<'a>(
        &self,
        code: Code,
        input: &'a [u8],
    ) -> Result<(Checked<&'a [u8]>, &'a [u8]), Error>;

    /// Are the given input bytes valid w.r.t. the code?
    fn is_valid_bytes(&self, code: Code, value: Checked<&[u8]>) -> bool;

    /// Decode the string value and encode it into the buffer.
    fn transcode_str(
        &self,
        prefix: &str,
        value: Checked<&str>,
        buf: &mut dyn Buffer,
    ) -> Result<(), Error>;

    /// Decode the bytes value and encode it into the formatter.
    fn transcode_bytes(
        &self,
        code: Code,
        value: Checked<&[u8]>,
        f: &mut fmt::Formatter,
    ) -> Result<(), Error>;
}

/// A type that can be extended with byte slices.
pub trait Buffer: AsRef<[u8]> {
    fn extend_with(&mut self, buf: &[u8]);
}

impl Buffer for alloc::vec::Vec<u8> {
    fn extend_with(&mut self, buf: &[u8]) {
        self.extend_from_slice(buf)
    }
}

impl<A: tinyvec::Array<Item = u8>> Buffer for TinyVec<A> {
    fn extend_with(&mut self, buf: &[u8]) {
        self.extend_from_slice(buf)
    }
}

/// Asserts that the wrapped value has been subject to some inspection.
///
/// Checked values are usually produced by codecs and ensure that certain
/// protocol specific premisses are fulfilled by the inner value. It is
/// safe to pass checked values to methods of the [`Protocol`] trait.
///
/// NB: For extensibility reasons checked values can be created by anyone,
/// but unless you know the specific checks that a particular protocol
/// requires you should better only pass checked values received from a
/// codec to a protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checked<T>(pub T);

impl<T> Deref for Checked<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A numeric protocol code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(u32);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Code {
    pub const fn new(n: u32) -> Self {
        Code(n)
    }
}

impl From<Code> for u32 {
    fn from(c: Code) -> Self {
        c.0
    }
}

/// Protocol value bytes.
#[derive(Debug, Clone)]
pub struct ProtoValue<'a> {
    code: Code,
    data: Bytes<'a>,
}

#[derive(Debug, Clone)]
enum Bytes<'a> {
    Slice(Checked<&'a [u8]>),
    Vector(Checked<TinyVec<[u8; 24]>>),
}

impl<'a> ProtoValue<'a> {
    /// Get the protocol code of this value.
    pub fn code(&self) -> Code {
        self.code
    }

    /// Get the checked data.
    pub fn data(&self) -> Checked<&[u8]> {
        match &self.data {
            Bytes::Slice(s) => *s,
            Bytes::Vector(v) => Checked(v),
        }
    }

    /// Try to convert into a typed protocol value.
    pub fn convert<P: Protocol<'a>>(&'a self) -> Result<P, Error> {
        P::read_bytes(self.data())
    }
}

impl<'a> AsRef<[u8]> for ProtoValue<'a> {
    fn as_ref(&self) -> &[u8] {
        &self.data()
    }
}

/// A sequence of [`Protocol`]s.
#[derive(Debug, Clone)]
pub struct MultiAddr {
    dat: TinyVec<[u8; 24]>,
    reg: Registry,
}

impl Default for MultiAddr {
    fn default() -> Self {
        MultiAddr::new(DEFAULT_REGISTRY.clone())
    }
}

impl PartialEq for MultiAddr {
    fn eq(&self, other: &Self) -> bool {
        self.dat.eq(&other.dat)
    }
}

impl Eq for MultiAddr {}

impl MultiAddr {
    /// Create an empty address with an explicit protocol codec registry.
    pub fn new(r: Registry) -> Self {
        MultiAddr {
            dat: TinyVec::new(),
            reg: r,
        }
    }

    /// Try to parse the given string as a multi-address.
    ///
    /// Alternative to the corresponding `TryFrom` impl, accepting an explicit
    /// protocol codec registry.
    pub fn try_from_str(input: &str, r: Registry) -> Result<Self, Error> {
        let iter = iter::StrIter::with_registry(input, r.clone());
        let mut b = TinyVec::new();
        for pair in iter {
            let (prefix, value) = pair?;
            let codec = r.get_by_prefix(prefix).unwrap();
            codec.transcode_str(prefix, value, &mut b)?;
        }
        Ok(MultiAddr { dat: b, reg: r })
    }

    /// Try to decode the given bytes as a multi-address.
    ///
    /// Alternative to the corresponding `TryFrom` impl, accepting an explicit
    /// protocol codec registry.
    pub fn try_from_bytes(input: &[u8], r: Registry) -> Result<Self, Error> {
        let iter = iter::BytesIter::with_registry(input, r.clone());
        let mut b = TinyVec::new();
        for item in iter {
            let (_, code, value) = item?;
            let codec = r.get_by_code(code).unwrap();
            if !codec.is_valid_bytes(code, value) {
                return Err(Error::invalid_proto(code));
            }
        }
        b.extend_from_slice(input);
        Ok(MultiAddr { dat: b, reg: r })
    }

    /// Does this multi-address contain any protocol components?
    pub fn is_empty(&self) -> bool {
        self.dat.is_empty()
    }

    /// Address length in bytes.
    pub fn len(&self) -> usize {
        self.dat.len()
    }

    /// Add a protocol to the end of this address.
    pub fn push_back<'a, P: Protocol<'a>>(&mut self, p: P) -> Result<(), Error> {
        if self.reg.get_by_code(P::CODE).is_none() {
            return Err(Error::unregistered(P::CODE));
        }
        debug_assert!(self.reg.get_by_prefix(P::PREFIX).is_some());
        p.write_bytes(&mut self.dat);
        Ok(())
    }

    /// Remove and return the last protocol component.
    ///
    /// O(n) in the number of protocols.
    pub fn pop_back<'a, 'b>(&'a mut self) -> Option<ProtoValue<'b>> {
        let iter = ValidBytesIter(iter::BytesIter::with_registry(&self.dat, self.reg.clone()));
        if let Some((o, c, Checked(p))) = iter.last() {
            debug_assert!(self.dat.ends_with(p));
            let dlen = self.dat.len();
            let plen = p.len();
            let val = split_off(&mut self.dat, dlen - plen);
            self.dat.truncate(o);
            Some(ProtoValue {
                code: c,
                data: Bytes::Vector(Checked(val)),
            })
        } else {
            None
        }
    }

    /// Remove the last protocol component.
    ///
    /// O(n) in the number of protocols.
    pub fn drop_last(&mut self) {
        let iter = ValidBytesIter(iter::BytesIter::with_registry(&self.dat, self.reg.clone()));
        if let Some((o, _, _)) = iter.last() {
            self.dat.truncate(o)
        }
    }

    /// Return a reference to the last protocol component.
    ///
    /// O(n) in the number of protocols.
    pub fn last(&self) -> Option<ProtoValue> {
        self.iter().last()
    }

    /// Get an iterator over the protocol components.
    pub fn iter(&self) -> ProtoIter {
        ProtoIter(ValidBytesIter(iter::BytesIter::with_registry(
            &self.dat,
            self.reg.clone(),
        )))
    }

    /// Drop any excess capacity.
    pub fn shrink_to_fit(&mut self) {
        self.dat.shrink_to_fit()
    }
}

impl fmt::Display for MultiAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for proto in self.iter() {
            let codec = self.reg.get_by_code(proto.code()).expect("valid code");
            if let Err(e) = codec.transcode_bytes(proto.code(), proto.data(), f) {
                if let error::ErrorImpl::Format(e) = e.into_impl() {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl TryFrom<&str> for MultiAddr {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        MultiAddr::try_from_str(value, DEFAULT_REGISTRY.clone())
    }
}

impl TryFrom<&[u8]> for MultiAddr {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        MultiAddr::try_from_bytes(value, DEFAULT_REGISTRY.clone())
    }
}

impl AsRef<[u8]> for MultiAddr {
    fn as_ref(&self) -> &[u8] {
        &self.dat
    }
}

impl From<MultiAddr> for alloc::vec::Vec<u8> {
    fn from(ma: MultiAddr) -> Self {
        match ma.dat {
            TinyVec::Heap(v) => v,
            TinyVec::Inline(a) => a.to_vec(),
        }
    }
}

/// Iterator over binary [`Protocol`] values of a [`MultiAddr`].
#[derive(Debug)]
pub struct ProtoIter<'a>(ValidBytesIter<'a>);

impl<'a> Iterator for ProtoIter<'a> {
    type Item = ProtoValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, c, p)| ProtoValue {
            code: c,
            data: Bytes::Slice(p),
        })
    }
}

// This iterator is only constructed from a MutiAddr value, hence
// the protocol parts are valid by construction and we expect them to be.
#[derive(Debug)]
struct ValidBytesIter<'a>(iter::BytesIter<'a>);

impl<'a> Iterator for ValidBytesIter<'a> {
    type Item = (usize, Code, Checked<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|x| x.expect("valid protocol"))
    }
}

/// Like [`TinyVec::split_off`] but attempts to inline data.
fn split_off<A>(v: &mut TinyVec<A>, at: usize) -> TinyVec<A>
where
    A: Array<Item = u8>,
{
    match v {
        TinyVec::Inline(a) => TinyVec::Inline(a.split_off(at)),
        TinyVec::Heap(v) => {
            if v.len() - at <= A::CAPACITY {
                let mut a = ArrayVec::default();
                a.extend_from_slice(&v[at..]);
                v.truncate(at);
                TinyVec::Inline(a)
            } else {
                TinyVec::Heap(v.split_off(at))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{proto, MultiAddr, Protocol};
    use std::net::Ipv4Addr;
    use tinyvec::TinyVec;

    #[test]
    fn one() {
        let mut ma = MultiAddr::default();
        ma.push_back(Ipv4Addr::from([127, 0, 0, 1])).unwrap();
        ma.push_back(Ipv4Addr::from([192, 168, 0, 1])).unwrap();

        for proto in ma.iter() {
            match proto.code() {
                Ipv4Addr::CODE => println!("{}", Ipv4Addr::read_bytes(proto.data()).unwrap()),
                code => println!("unknown code {code}"),
            }
        }
    }

    #[test]
    fn two() {
        let mut ma = MultiAddr::try_from(
            "/ip4/127.0.0.1/tcp/80/ip4/192.168.0.1/ip6/::1/tcp/443/dns/example.com/tcp/80",
        )
        .unwrap();
        println!("> {}", ma);
        let _tcp: proto::Tcp = ma.pop_back().unwrap().convert().unwrap();
        println!("> {}", ma);
        while !ma.is_empty() {
            ma.pop_back();
            println!("> {}", ma)
        }
    }

    #[test]
    fn three() {
        let ma = MultiAddr::try_from(
            "/ip4/127.0.0.1/tcp/80/ip4/192.168.0.1/ip6/::1/tcp/443/dns/example.com/tcp/80",
        )
        .unwrap();
        let vec: Vec<u8> = ma.into();
        MultiAddr::try_from(vec.as_slice()).unwrap();
    }

    #[test]
    fn four() {
        let mut t: TinyVec<[u8; 5]> = TinyVec::new();
        t.extend_from_slice(b"hello");
        assert!(t.is_inline());
        t.extend_from_slice(b"world");
        assert!(t.is_heap());
        let mut v = t.clone();
        let a = v.split_off(5);
        assert!(a.is_heap());
        let b = super::split_off(&mut t, 5);
        assert!(b.is_inline());
        assert_eq!(a, b);
        assert_eq!(v, t);
    }
}
