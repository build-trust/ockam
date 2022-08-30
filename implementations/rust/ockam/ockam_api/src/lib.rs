pub mod auth;
pub mod authenticator;
pub mod cloud;
pub mod config;
pub mod echoer;
pub mod error;
pub mod identity;
pub mod nodes;
pub mod uppercase;
pub mod vault;
pub mod verifier;

mod util;
pub use util::*;

#[cfg(feature = "lmdb")]
pub mod lmdb;

#[macro_use]
extern crate tracing;

use core::fmt;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(rust_embed::RustEmbed)]
#[folder = "./static"]
pub(crate) struct StaticFiles;

/// Newtype around [`Vec<u8>`] that provides base-16 string encoding using serde.
#[derive(Debug, Clone, Default, Encode, Decode)]
#[cbor(transparent)]
pub struct HexByteVec(#[b(0)] pub Vec<u8>);

impl HexByteVec {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for HexByteVec {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<HexByteVec> for Vec<u8> {
    fn from(h: HexByteVec) -> Self {
        h.0
    }
}

impl Serialize for HexByteVec {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::serde::serialize(&*self.0, s)
        } else {
            s.serialize_bytes(&*self.0)
        }
    }
}

impl<'de> Deserialize<'de> for HexByteVec {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let v: Vec<u8> = hex::serde::deserialize(d)?;
            Ok(Self(v))
        } else {
            let v = <Vec<u8>>::deserialize(d)?;
            Ok(Self(v))
        }
    }
}

impl fmt::Display for HexByteVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}
