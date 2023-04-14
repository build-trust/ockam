use crate::api::{DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse};
use crate::channel::addresses::Addresses;
use crate::channel::decryptor::Decryptor;
use crate::error::IdentityError;
use crate::{IdentityIdentifier, IdentitySecureChannelLocalInfo};
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Address, Decodable, Encodable, LocalMessage, Route};
use ockam_core::{Any, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::debug;
use tracing::warn;

pub(crate) struct DecryptorWorker {
    //for debug purposes only
    pub(crate) role: &'static str,
    pub(crate) addresses: Addresses,
    pub(crate) decryptor: Decryptor,
    pub(crate) their_identity_id: IdentityIdentifier,
}

impl DecryptorWorker {
    pub fn new(
        role: &'static str,
        addresses: Addresses,
        decryptor: Decryptor,
        their_identity_id: IdentityIdentifier,
    ) -> Self {
        Self {
            role,
            addresses,
            decryptor,
            their_identity_id,
        }
    }

    async fn handle_decrypt_api(
        &mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt API {}",
            self.role, &self.addresses.decryptor_remote
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = DecryptionRequest::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&request.0).await;

        let response = match decrypted_payload {
            Ok(payload) => DecryptionResponse::Ok(payload),
            Err(err) => DecryptionResponse::Err(err),
        };

        // Send reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.decryptor_api.clone())
            .await?;

        Ok(())
    }

    async fn handle_decrypt(
        &mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt {}",
            self.role, &self.addresses.decryptor_remote
        );

        // Decode raw payload binary
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&payload).await?;

        // Encrypted data should be a TransportMessage
        let mut transport_message = TransportMessage::decode(&decrypted_payload)?;

        // Add encryptor hop in the return_route (instead of our address)
        transport_message
            .return_route
            .modify()
            .prepend(self.addresses.encryptor.clone());

        // Mark message LocalInfo with IdentitySecureChannelLocalInfo,
        // replacing any pre-existing entries
        let local_info =
            IdentitySecureChannelLocalInfo::mark(vec![], self.their_identity_id.clone())?;

        let msg = LocalMessage::new(transport_message, local_info);

        match ctx
            .forward_from_address(msg, self.addresses.decryptor_internal.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    "{} forwarding decrypted message from {}",
                    err, &self.addresses.encryptor
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl Worker for DecryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.addresses.decryptor_remote {
            self.handle_decrypt(ctx, msg).await?;
        } else if msg_addr == self.addresses.decryptor_api {
            self.handle_decrypt_api(ctx, msg).await?;
        } else {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        Ok(())
    }
}
