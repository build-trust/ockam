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
use ockam_core::compat::borrow::Cow;
use ockam_core::CowBytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(rust_embed::RustEmbed)]
#[folder = "./static"]
pub(crate) struct StaticFiles;

/// Newtype around [`CowBytes`] that provides base-16 string encoding using serde.
#[derive(Debug, Clone, Default, Encode, Decode)]
#[cbor(transparent)]
pub struct HexBytes<'a>(#[b(0)] pub CowBytes<'a>);

impl<'a> HexBytes<'a> {
    pub fn new<B: Into<Cow<'a, [u8]>>>(b: B) -> Self {
        Self(CowBytes(b.into()))
    }

    pub fn to_owned<'r>(&self) -> HexBytes<'r> {
        HexBytes(self.0.to_owned())
    }
}

impl Serialize for HexBytes<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            hex::serde::serialize(&*self.0, s)
        } else {
            s.serialize_bytes(&*self.0)
        }
    }
}

impl<'de> Deserialize<'de> for HexBytes<'de> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let v: Vec<u8> = hex::serde::deserialize(d)?;
            Ok(Self(CowBytes(v.into())))
        } else {
            let v = <&'de [u8]>::deserialize(d)?;
            Ok(Self(CowBytes(v.into())))
        }
    }
}

impl fmt::Display for HexBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}
