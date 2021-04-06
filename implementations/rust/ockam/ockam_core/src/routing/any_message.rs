use crate::{Message, Encoded, Result};

/// Any
pub struct AnyMessage;

impl Message for AnyMessage {
    fn encode(&self) -> Result<Encoded> {
        Ok(Encoded::new())
    }

    fn decode(_e: &Encoded) -> Result<Self> {
        Ok(Self)
    }
}