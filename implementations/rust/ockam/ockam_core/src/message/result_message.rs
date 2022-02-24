use crate::{Message, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

impl<M: Message + Serialize + DeserializeOwned> Message for ResultMessage<M> {}

/// A `ResultMessage` is used between workers when Error-handling is needed.
#[derive(Serialize, Deserialize)]
pub struct ResultMessage<M: Message>(Result<M>);

impl<M> ResultMessage<M>
where
    M: Message,
{
    /// Creates a new `ResultMessage<R>` suitable for holding a value of type `M`.
    pub fn new(inner: Result<M>) -> Self {
        Self(inner)
    }
}

impl<M: Message> From<Result<M>> for ResultMessage<M> {
    fn from(other: Result<M>) -> Self {
        Self::new(other)
    }
}

#[allow(clippy::from_over_into)]
impl<M: Message> Into<Result<M>> for ResultMessage<M> {
    fn into(self) -> Result<M> {
        self.0
    }
}
