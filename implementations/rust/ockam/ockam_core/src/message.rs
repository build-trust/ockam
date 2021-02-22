use crate::{lib::Vec, Result};
use serde::{de::DeserializeOwned, Serialize};

// TODO: swap this for a non-heaped data structure
pub type Encoded = Vec<u8>;

/// A user defined message that can be serialised and deserialised
pub trait Message: Serialize + DeserializeOwned + Send + 'static {
    fn encode(&self) -> Result<Encoded> {
        Ok(bincode::serialize(self)?)
    }

    #[allow(clippy::ptr_arg)]
    fn decode(e: &Encoded) -> Result<Self> {
        Ok(bincode::deserialize(e)?)
    }
}

// Auto-implement message trait for types that _can_ be messages
impl<T> Message for T where T: Serialize + DeserializeOwned + Send + 'static {}

// TODO: see comment in Cargo.toml about this dependency
impl From<bincode::Error> for crate::Error {
    fn from(_: bincode::Error) -> Self {
        Self::new(1, "bincode")
    }
}
