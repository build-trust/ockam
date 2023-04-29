use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use minicbor::decode::{self, Decoder};
use minicbor::encode::{self, Encoder, Write};
use minicbor::{Decode, Encode};
use serde::Serialize;
use zeroize::Zeroize;

/// A type tag represents a type as a unique numeric value.
///
/// This zero-sized type is meant to help catching type errors in cases where
/// CBOR items structurally match various nominal types. It will end up as an
/// unsigned integer in CBOR and decoding checks that the value is expected.
#[derive(Clone, Copy, Default, Eq, Zeroize)]
pub struct TypeTag<const N: usize>;

// Custom `Debug` impl to include the tag number.
impl<const N: usize> fmt::Debug for TypeTag<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeTag").field(&N).finish()
    }
}

impl<const N: usize> Hash for TypeTag<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(N)
    }
}

impl<const N: usize> PartialEq for TypeTag<N> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<const N: usize> PartialOrd for TypeTag<N> {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl<const N: usize> Ord for TypeTag<N> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<C, const N: usize> Encode<C> for TypeTag<N> {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.u64(N as u64)?.ok()
    }
}

impl<'b, C, const N: usize> Decode<'b, C> for TypeTag<N> {
    fn decode(d: &mut Decoder<'b>, _: &mut C) -> Result<Self, decode::Error> {
        let n = d.u64()?;
        if N as u64 == n {
            return Ok(TypeTag);
        }
        let msg = format!("type tag mismatch (expected {N}, got {n})");
        Err(decode::Error::message(msg))
    }
}

impl<const N: usize> Serialize for TypeTag<N> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(N as u64)
    }
}
