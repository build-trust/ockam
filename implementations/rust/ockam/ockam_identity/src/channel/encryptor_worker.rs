use crate::api::{EncryptionRequest, EncryptionResponse};
use crate::channel::addresses::Addresses;
use crate::channel::encryptor::Encryptor;
use crate::error::IdentityError;
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Address, Decodable, Encodable, Route};
use ockam_core::{Any, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::debug;

pub(crate) struct EncryptorWorker {
    //for debug purposes only
    role: &'static str,
    addresses: Addresses,
    remote_route: Route,
    remote_backwards_compatibility_address: Address,
    encryptor: Encryptor,
}

impl EncryptorWorker {
    pub fn new(
        role: &'static str,
        addresses: Addresses,
        remote_route: Route,
        remote_backwards_compatibility_address: Address,
        encryptor: Encryptor,
    ) -> Self {
        Self {
            role,
            addresses,
            remote_route,
            remote_backwards_compatibility_address,
            encryptor,
        }
    }

    async fn handle_encrypt_api(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Encrypt API {}",
            self.role, &self.addresses.encryptor
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = EncryptionRequest::decode(&msg.into_transport_message().payload)?;

        // Encrypt the message
        let encrypted_payload = self.encryptor.encrypt(&request.0).await;

        let response = match encrypted_payload {
            Ok(payload) => EncryptionResponse::Ok(payload),
            Err(err) => EncryptionResponse::Err(err),
        };

        // Send the reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.encryptor_api.clone())
            .await?;

        Ok(())
    }

    async fn handle_encrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Encrypt {}",
            self.role, &self.addresses.encryptor
        );

        let mut onward_route = msg.onward_route();
        let return_route = msg.return_route();

        // Remove our address
        let _ = onward_route.step();

        // Add backwards compatibility address to simulate old behaviour where secure channel
        // and identity secure channel workers were separate
        onward_route
            .modify()
            .prepend(self.remote_backwards_compatibility_address.clone());

        let msg = TransportMessage::v1(
            onward_route,
            return_route,
            msg.into_transport_message().payload,
        );

        // Encrypt the message
        let encrypted_payload = self.encryptor.encrypt(&msg.encode()?).await?;

        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            self.remote_route.clone(),
            encrypted_payload,
            self.addresses.encryptor.clone(),
        )
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
        let msg_addr = msg.msg_addr();

        if msg_addr == self.addresses.encryptor {
            self.handle_encrypt(ctx, msg).await?;
        } else if msg_addr == self.addresses.encryptor_api {
            self.handle_encrypt_api(ctx, msg).await?;
        } else {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        Ok(())
    }
}
