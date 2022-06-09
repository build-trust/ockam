use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{Address, Any, LocalMessage, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::debug;

pub(crate) struct EncryptorWorker {
    is_initiator: bool,
    remote_identity_secure_channel_address: Address,
    local_secure_channel_address: Address,
}

impl EncryptorWorker {
    pub fn new(
        is_initiator: bool,
        remote_identity_secure_channel_address: Address,
        local_secure_channel_address: Address,
    ) -> Self {
        Self {
            is_initiator,
            remote_identity_secure_channel_address,
            local_secure_channel_address,
        }
    }

    async fn handle_encrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "IdentitySecureChannel {} received Encrypt",
            if self.is_initiator {
                "Initiator"
            } else {
                "Responder"
            }
        );

        let mut onward_route = msg.onward_route();
        let return_route = msg.return_route();
        let payload = msg.payload().to_vec();

        // Send to the other party using local regular SecureChannel
        let _ = onward_route.step()?;
        let onward_route = onward_route
            .modify()
            .prepend(self.remote_identity_secure_channel_address.clone())
            .prepend(self.local_secure_channel_address.clone());

        let transport_msg = TransportMessage::v1(onward_route, return_route, payload);

        ctx.forward(LocalMessage::new(transport_msg, Vec::new()))
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for EncryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.handle_encrypt(ctx, msg).await
    }
}
