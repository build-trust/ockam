use crate::channel_types::small_channel;
use crate::{error::*, Context, NodeMessage};
use ockam_core::{Address, Result, TransportType};

impl Context {
    // TODO: This method should be deprecated
    /// Register a router for a specific address type
    pub async fn register<A: Into<Address>>(&self, type_: TransportType, addr: A) -> Result<()> {
        self.register_impl(type_, addr.into()).await
    }

    async fn register_impl(&self, type_: TransportType, addr: Address) -> Result<()> {
        let (tx, mut rx) = small_channel();
        self.sender
            .send(NodeMessage::Router(type_, addr, tx))
            .await
            .map_err(NodeError::from_send_err)?;

        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }
}
