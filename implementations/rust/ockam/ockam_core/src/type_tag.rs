use core::fmt;
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
#[derive(Clone, Copy, Default, PartialEq, Eq, Zeroize)]
pub struct TypeTag<const N: usize>;

// Custom `Debug` impl to include the tag number.
impl<const N: usize> fmt::Debug for TypeTag<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeTag").field(&N).finish()
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
