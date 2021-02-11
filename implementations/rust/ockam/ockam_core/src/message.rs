use crate::Result;
use serde::{de::DeserializeOwned, Serialize};

// TODO: swap this for a non-heaped data structure
pub type Encoded = Vec<u8>;

/// A user defined message that can be serialised and deserialised
pub trait Message: Serialize + DeserializeOwned + Send + 'static {
    fn encode(&self) -> Result<Encoded> {
        Ok(bincode::serialize(self)?)
    }
    fn decode(e: &Encoded) -> Result<Self> {
        Ok(bincode::deserialize(e)?)
    }
}

// TODO: see comment in Cargo.toml about this dependency
impl From<bincode::Error> for crate::Error {
    fn from(_: bincode::Error) -> Self {
        Self::new(1, "bincode")
    }
}
