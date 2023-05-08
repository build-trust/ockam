//! This crate provides an implementation of multiformats.io/multiaddr.
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

use alloc::vec::Vec;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use core::str::FromStr;
use once_cell::race::OnceBox;
use std::net::{SocketAddrV4, SocketAddrV6};
use tinyvec::{Array, ArrayVec, TinyVec};

use crate::proto::{DnsAddr, Ip4, Ip6, Tcp};
pub use error::Error;
use ockam_core::env::FromString;
pub use registry::{Registry, RegistryBuilder};

/// Global default registry of known protocols.
fn default_registry() -> &'static Registry {
    static INSTANCE: OnceBox<Registry> = OnceBox::new();
    INSTANCE.get_or_init(Box::<Registry>::default)
}

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

    /// Write a protocol value to the given buffer.
    fn write_bytes(&self, val: &ProtoValue, buf: &mut dyn Buffer) -> Result<(), Error>;

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

impl Buffer for Vec<u8> {
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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    Owned(Checked<TinyVec<[u8; 28]>>),
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
            Bytes::Owned(v) => Checked(v),
        }
    }

    /// Convert to a typed protocol value.
    pub fn cast<P: Protocol<'a>>(&'a self) -> Option<P> {
        if self.code != P::CODE {
            return None;
        }
        P::read_bytes(self.data()).ok()
    }

    /// Clone an owned value of this type.
    pub fn to_owned<'b>(&self) -> ProtoValue<'b> {
        match &self.data {
            Bytes::Slice(Checked(s)) => ProtoValue {
                code: self.code,
                data: Bytes::Owned(Checked(TinyVec::Heap(Vec::from(*s)))),
            },
            Bytes::Owned(Checked(v)) => ProtoValue {
                code: self.code,
                data: Bytes::Owned(Checked(v.clone())),
            },
        }
    }
}

impl<'a> AsRef<[u8]> for ProtoValue<'a> {
    fn as_ref(&self) -> &[u8] {
        &self.data()
    }
}

/// A sequence of [`Protocol`]s.
#[derive(Debug)]
pub struct MultiAddr {
    dat: TinyVec<[u8; 28]>,
    off: usize,
    reg: Registry,
}

impl Default for MultiAddr {
    fn default() -> Self {
        MultiAddr::new(default_registry().clone())
    }
}

impl PartialEq for MultiAddr {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl Eq for MultiAddr {}

impl Hash for MultiAddr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl Clone for MultiAddr {
    fn clone(&self) -> Self {
        if self.off > 0 {
            // do not copy unused prefix
            MultiAddr {
                dat: match &self.dat {
                    TinyVec::Inline(a) => TinyVec::Inline({
                        let mut b = ArrayVec::default();
                        b.extend_from_slice(&a[self.off..]);
                        b
                    }),
                    TinyVec::Heap(v) => TinyVec::Heap({
                        let mut w = Vec::with_capacity(v.len() - self.off);
                        w.extend_from_slice(&v[self.off..]);
                        w
                    }),
                },
                off: 0,
                reg: self.reg.clone(),
            }
        } else {
            MultiAddr {
                dat: self.dat.clone(),
                off: self.off,
                reg: self.reg.clone(),
            }
        }
    }
}

impl FromString for MultiAddr {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Self::from_str(s).map_err(|_| {
            ockam_core::Error::new(
                ockam_core::errcode::Origin::Core,
                ockam_core::errcode::Kind::Internal,
                "MultiAddr parse error",
            )
        })
    }
}

impl MultiAddr {
    /// Create an empty address with an explicit protocol codec registry.
    pub fn new(r: Registry) -> Self {
        MultiAddr {
            dat: TinyVec::new(),
            off: 0,
            reg: r,
        }
    }

    /// Access the registry of this `MultiAddr`.
    pub fn registry(&self) -> &Registry {
        &self.reg
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
            let codec = r
                .get_by_prefix(prefix)
                .ok_or_else(|| Error::unregistered_prefix(prefix))?;
            codec.transcode_str(prefix, value, &mut b)?;
        }
        Ok(MultiAddr {
            dat: b,
            off: 0,
            reg: r,
        })
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
            let codec = r
                .get_by_code(code)
                .ok_or_else(|| Error::unregistered(code))?;
            if !codec.is_valid_bytes(code, value) {
                return Err(Error::invalid_proto(code));
            }
        }
        b.extend_from_slice(input);
        Ok(MultiAddr {
            dat: b,
            off: 0,
            reg: r,
        })
    }

    /// Try to decode the given CBOR bytes as a multi-address.
    ///
    /// Alternative to the `minicbor::Decode` implementation, accepting an
    /// explicit codec registry.
    #[cfg(feature = "cbor")]
    pub fn try_from_cbor(input: &[u8], r: Registry) -> Result<Self, Error> {
        let bytes = minicbor::Decoder::new(input)
            .bytes()
            .map_err(|e| Error::message(format!("invalid cbor: {e}")))?;
        Self::try_from_bytes(bytes, r)
    }

    /// Does this multi-address contain any protocol components?
    pub fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    /// Address length in bytes.
    pub fn len(&self) -> usize {
        self.as_ref().len()
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

    /// Add a protocol value to the end of this address.
    pub fn push_back_value(&mut self, p: &ProtoValue) -> Result<(), Error> {
        if let Some(codec) = self.reg.get_by_code(p.code()) {
            codec.write_bytes(p, &mut self.dat)
        } else {
            Err(Error::unregistered(p.code()))
        }
    }

    /// Add a protocol to the front of this address.
    pub fn push_front<'a, P: Protocol<'a>>(&mut self, p: P) -> Result<(), Error> {
        if self.reg.get_by_code(P::CODE).is_none() {
            return Err(Error::unregistered(P::CODE));
        }
        debug_assert!(self.reg.get_by_prefix(P::PREFIX).is_some());
        let mut dat = TinyVec::new();
        p.write_bytes(&mut dat);
        dat.extend_from_slice(&self.dat); // TODO
        self.dat = dat;
        Ok(())
    }

    /// Add a protocol value to the front of this address.
    pub fn push_front_value(&mut self, p: &ProtoValue) -> Result<(), Error> {
        if let Some(codec) = self.reg.get_by_code(p.code()) {
            let mut dat = TinyVec::new();
            codec.write_bytes(p, &mut dat)?;
            dat.extend_from_slice(&self.dat); // TODO
            self.dat = dat;
            Ok(())
        } else {
            Err(Error::unregistered(p.code()))
        }
    }

    /// Remove and return the last protocol component.
    ///
    /// O(n) in the number of protocols.
    pub fn pop_back<'b>(&mut self) -> Option<ProtoValue<'b>> {
        let iter = ValidBytesIter(iter::BytesIter::with_registry(
            &self.dat[self.off..],
            self.reg.clone(),
        ));
        if let Some((o, c, Checked(p))) = iter.last() {
            debug_assert!(self.dat.ends_with(p));
            let dlen = self.len();
            let plen = p.len();
            let val = split_off(&mut self.dat, self.off + dlen - plen);
            self.dat.truncate(self.off + o);
            Some(ProtoValue {
                code: c,
                data: Bytes::Owned(Checked(val)),
            })
        } else {
            None
        }
    }

    /// Remove and return the first protocol component.
    pub fn pop_front(&mut self) -> Option<ProtoValue> {
        let mut iter = ValidBytesIter(iter::BytesIter::with_registry(
            &self.dat[self.off..],
            self.reg.clone(),
        ));
        if let Some((_, c, Checked(p))) = iter.next() {
            self.off += iter.0.offset();
            let val = &self.dat[self.off - p.len()..self.off];
            debug_assert_eq!(val, p);
            Some(ProtoValue {
                code: c,
                data: Bytes::Slice(Checked(val)),
            })
        } else {
            None
        }
    }

    /// Remove the first protocol component.
    pub fn drop_first(&mut self) {
        let mut iter = ValidBytesIter(iter::BytesIter::with_registry(
            self.as_ref(),
            self.reg.clone(),
        ));
        if iter.next().is_some() {
            self.off += iter.0.offset()
        }
    }

    /// Remove the last protocol component.
    ///
    /// O(n) in the number of protocols.
    pub fn drop_last(&mut self) {
        let iter = ValidBytesIter(iter::BytesIter::with_registry(
            self.as_ref(),
            self.reg.clone(),
        ));
        if let Some((o, _, _)) = iter.last() {
            self.dat.truncate(self.off + o)
        }
    }

    /// Return a reference to the first protocol component.
    pub fn first(&self) -> Option<ProtoValue> {
        self.iter().next()
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
            self.as_ref(),
            self.reg.clone(),
        )))
    }

    /// Drop any excess capacity.
    pub fn shrink_to_fit(&mut self) {
        self.dat.shrink_to_fit()
    }

    /// Try to extend this multi-address with another sequence of protocols.
    pub fn try_extend<'a, T>(&mut self, iter: T) -> Result<(), Error>
    where
        T: IntoIterator<Item = ProtoValue<'a>>,
    {
        for p in iter.into_iter() {
            self.push_back_value(&p)?
        }
        Ok(())
    }

    /// Like `try_extend` but moves `Self`.
    pub fn try_with<'a, T>(mut self, iter: T) -> Result<Self, Error>
    where
        T: IntoIterator<Item = ProtoValue<'a>>,
    {
        self.try_extend(iter)?;
        Ok(self)
    }

    /// Check if the protocol codes match the given sequence.
    pub fn matches<'a, I>(&self, start: usize, codes: I) -> bool
    where
        I: IntoIterator<Item = &'a Match>,
        I::IntoIter: ExactSizeIterator,
    {
        let codes = codes.into_iter();
        let mut n = codes.len();
        for (p, c) in self.iter().skip(start).zip(codes) {
            n -= 1;
            match c {
                Match::Val(c) => {
                    if p.code() != *c {
                        return false;
                    }
                }
                Match::Any(cs) => {
                    if !cs.contains(&p.code()) {
                        return false;
                    }
                }
            }
        }
        n == 0
    }

    pub fn split(&self, at: usize) -> (MultiAddr, MultiAddr) {
        let mut iter = self.iter();
        let a = MultiAddr::default()
            .try_with((&mut iter).take(at))
            .expect("valid address");
        let b = MultiAddr::default().try_with(iter).expect("valid address");
        (a, b)
    }

    pub fn concat_mut(&mut self, other: &MultiAddr) -> Result<(), Error> {
        for proto in other.iter() {
            self.push_back_value(&proto)?;
        }

        Ok(())
    }

    pub fn concat(self, other: &MultiAddr) -> Result<MultiAddr, Error> {
        let mut addr = self;

        addr.concat_mut(other)?;

        Ok(addr)
    }

    /// If the input MultiAddr is "/dnsaddr/localhost/tcp/4000/service/api",
    /// then this will return string format of the SocketAddr: "127.0.0.1:4000".
    pub fn to_socket_addr(&self) -> Result<String, Error> {
        let mut it = self.iter().peekable();
        while let Some(p) = it.next() {
            match p.code() {
                Ip4::CODE => {
                    let ip4 = p.cast::<Ip4>().unwrap();
                    let port = it.next().unwrap().cast::<Tcp>().unwrap();
                    return Ok(SocketAddrV4::new(*ip4, *port).to_string());
                }
                Ip6::CODE => {
                    let ip6 = p.cast::<Ip6>().unwrap();
                    let port = it.next().unwrap().cast::<Tcp>().unwrap();
                    return Ok(SocketAddrV6::new(*ip6, *port, 0, 0).to_string());
                }
                DnsAddr::CODE => {
                    let host = p.cast::<DnsAddr>().unwrap();
                    if let Some(p) = it.peek() {
                        if p.code() == Tcp::CODE {
                            let port = p.cast::<Tcp>().unwrap();
                            return Ok(format!("{}:{}", &*host, *port));
                        }
                    }
                }
                other => {
                    return Err(Error::invalid_proto(other));
                }
            }
        }
        Err(Error::message("No socket address found"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    Val(Code),
    Any(TinyVec<[Code; 4]>),
}

impl Match {
    pub fn code(c: Code) -> Self {
        Self::Val(c)
    }

    pub fn any<I: IntoIterator<Item = Code>>(cs: I) -> Self {
        Self::Any(cs.into_iter().collect())
    }
}

impl From<Code> for Match {
    fn from(c: Code) -> Self {
        Match::code(c)
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

impl<'a> IntoIterator for &'a MultiAddr {
    type Item = ProtoValue<'a>;
    type IntoIter = ProtoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl TryFrom<&str> for MultiAddr {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        MultiAddr::try_from_str(value, default_registry().clone())
    }
}

impl TryFrom<&[u8]> for MultiAddr {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        MultiAddr::try_from_bytes(value, default_registry().clone())
    }
}

impl FromStr for MultiAddr {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl AsRef<[u8]> for MultiAddr {
    fn as_ref(&self) -> &[u8] {
        &self.dat[self.off..]
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MultiAddr {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.serialize_str(&self.to_string())
        } else {
            s.serialize_bytes(self.as_ref())
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MultiAddr {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let s = <&'de str>::deserialize(d)?;
            MultiAddr::try_from(s).map_err(serde::de::Error::custom)
        } else {
            let b = <&'de [u8]>::deserialize(d)?;
            MultiAddr::try_from(b).map_err(serde::de::Error::custom)
        }
    }
}

#[cfg(feature = "cbor")]
impl<C> minicbor::Encode<C> for MultiAddr {
    fn encode<W>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.bytes(self.as_ref())?.ok()
    }
}

#[cfg(feature = "cbor")]
impl<'b, C> minicbor::Decode<'b, C> for MultiAddr {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        MultiAddr::try_from(d.bytes()?)
            .map_err(|e| minicbor::decode::Error::message(format!("{e}")))
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
    use tinyvec::TinyVec;

    #[test]
    fn split_off() {
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
