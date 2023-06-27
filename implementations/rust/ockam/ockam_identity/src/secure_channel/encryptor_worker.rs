use crate::identity::IdentityError;
use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::api::{EncryptionRequest, EncryptionResponse};
use crate::secure_channel::encryptor::Encryptor;
use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Decodable, Encodable, Route};
use ockam_core::{Any, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::{debug, error};

pub(crate) struct EncryptorWorker {
    //for debug purposes only
    role: &'static str,
    addresses: Addresses,
    remote_route: Route,
    encryptor: Encryptor,
}

impl EncryptorWorker {
    pub fn new(
        role: &'static str,
        addresses: Addresses,
        remote_route: Route,
        encryptor: Encryptor,
    ) -> Self {
        Self {
            role,
            addresses,
            remote_route,
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

        let mut should_stop = false;

        // Encrypt the message
        let response = match self.encryptor.encrypt(&request.0).await {
            Ok(encrypted_payload) => EncryptionResponse::Ok(encrypted_payload),
            // If encryption failed, that means we have some internal error,
            // and we may be in an invalid state, it's better to stop the Worker
            Err(err) => {
                should_stop = true;
                error!(
                    "Error while encrypting: {err} at: {}",
                    self.addresses.encryptor
                );
                EncryptionResponse::Err(err)
            }
        };

        // Send the reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.encryptor_api.clone())
            .await?;

        if should_stop {
            ctx.stop_worker(self.addresses.encryptor.clone()).await?;
        }

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

        let msg = TransportMessage::v1(
            onward_route,
            return_route,
            msg.into_transport_message().payload,
        );

        // Encrypt the message
        let encrypted_payload = match self.encryptor.encrypt(&msg.encode()?).await {
            Ok(encrypted_payload) => encrypted_payload,
            // If encryption failed, that means we have some internal error,
            // and we may be in an invalid state, it's better to stop the Worker
            Err(err) => {
                let address = self.addresses.encryptor.clone();
                error!("Error while encrypting: {err} at: {address}");
                ctx.stop_worker(address).await?;
                return Ok(());
            }
        };

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

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.encryptor.shutdown().await
    }
}
