use crate::{Message, Result};
use serde::{Deserialize, Serialize};

/// Message that is meant to be sent between workers if Error-handling is needed.
#[derive(Serialize, Deserialize)]
pub struct ResultMessage<M: Message>(Result<M>);

impl<M> ResultMessage<M>
where
    M: Message,
{
    /// Constructor
    pub fn new(inner: Result<M>) -> Self {
        Self(inner)
    }
}

impl<M: Message> From<Result<M>> for ResultMessage<M> {
    fn from(other: Result<M>) -> Self {
        Self::new(other)
    }
}

impl<M: Message> Into<Result<M>> for ResultMessage<M> {
    fn into(self) -> Result<M> {
        self.0
    }
}
