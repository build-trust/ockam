use crate::context::ContextState;
use crate::{Context, MessageSendOptions, NodeError};
use ockam_core::compat::sync::{Arc, Weak};
use ockam_core::{Address, Result};
use ockam_core::{Message, Route};

/// Cloneable [`Context`] variant that allows shared access to sending capabilities from a specified
/// address. Doesn't own any mailbox, therefore original [`Context`] must be alive to be able
/// to send the message.
#[derive(Clone)]
pub struct ContextSend {
    primary_address: Address,
    state: Weak<ContextState>,
}

impl ContextSend {
    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// ```rust
    /// # use {ockam_node::Context, ockam_core::Result};
    /// use ockam_core::{deserialize, serialize, Decodable, Encodable, Encoded};
    ///
    /// async fn test(ctx: &mut Context) -> Result<()> {
    /// use ockam_core::Message;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Message, Serialize, Deserialize)]
    /// struct MyMessage(String);
    ///
    /// impl MyMessage {
    ///     fn new(s: &str) -> Self {
    ///         Self(s.into())
    ///     }
    /// }
    ///
    /// impl Encodable for MyMessage {
    ///     fn encode(self) -> Result<Encoded> {
    ///         Ok(serialize(self)?)
    ///     }
    /// }
    ///
    /// impl Decodable for MyMessage {
    ///     fn decode(e: &[u8]) -> Result<Self> {
    ///         Ok(deserialize(e)?)
    ///     }
    /// }
    ///
    /// ctx.send("my-test-worker", MyMessage::new("Hello you there :)")).await?;
    /// Ok(())
    /// # }
    /// ```
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message,
    {
        let state = self
            .state
            .upgrade()
            .ok_or(NodeError::SendUpgrade(self.primary_address.clone()).not_found())?;

        state
            .send(route.into(), msg, MessageSendOptions::new())
            .await
    }
}

impl Context {
    /// Get cloneable [`Context`] variant that allows shared access to sending capabilities from
    /// a specified address. Doesn't own any mailbox, therefore original [`Context`] must be alive
    /// to be able to send the message.
    pub fn get_sending_context(&self) -> ContextSend {
        ContextSend {
            primary_address: self.primary_address().clone(),
            state: Arc::downgrade(&self.state),
        }
    }
}
