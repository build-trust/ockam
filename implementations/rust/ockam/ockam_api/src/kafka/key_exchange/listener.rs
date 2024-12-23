use crate::DefaultAddress;
use minicbor::{CborLen, Decode, Encode};
use ockam::identity::{
    SecureChannelApiRequest, SecureChannelApiResponse, SecureChannelRegistry, TimestampInSeconds,
};
use ockam_core::compat::clock::Clock;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    async_trait, route, Address, Decodable, Encodable, Encoded, IncomingAccessControl, Message,
    OutgoingAccessControl, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_vault::{VaultForEncryptionAtRest, VaultForSecureChannels};
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct KafkaKeyExchangeListener {
    encryption_at_rest: Arc<dyn VaultForEncryptionAtRest>,
    secure_channel_vault: Arc<dyn VaultForSecureChannels>,
    secure_channel_registry: SecureChannelRegistry,
    rekey_period: Duration,
    key_validity: Duration,
    key_rotation: Duration,
    clock: Box<dyn Clock>,
}

#[derive(Debug, CborLen, Encode, Decode)]
#[rustfmt::skip]
pub(crate) struct KeyExchangeRequest {
    #[n(1)] pub local_decryptor_address: Address,
}

#[derive(Debug, CborLen, Encode, Decode)]
#[rustfmt::skip]
pub(crate) struct KeyExchangeResponse {
    #[n(0)] pub key_identifier_for_consumer: Vec<u8>,
    #[n(1)] pub valid_until: TimestampInSeconds,
    #[n(2)] pub rotate_after: TimestampInSeconds,
    #[n(3)] pub rekey_period: Duration,
}

impl Encodable for KeyExchangeRequest {
    fn encode(self) -> ockam_core::Result<Encoded> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for KeyExchangeRequest {
    fn decode(data: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}
impl Message for KeyExchangeRequest {}
impl Encodable for KeyExchangeResponse {
    fn encode(self) -> ockam_core::Result<Encoded> {
        ockam_core::cbor_encode_preallocate(self)
    }
}

impl Decodable for KeyExchangeResponse {
    fn decode(data: &[u8]) -> ockam_core::Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

impl Message for KeyExchangeResponse {}

#[async_trait]
impl Worker for KafkaKeyExchangeListener {
    type Context = Context;
    type Message = KeyExchangeRequest;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let request: KeyExchangeRequest = minicbor::decode(message.payload())?;
        let local_decryptor = Address::from_string(request.local_decryptor_address);

        let entry = self
            .secure_channel_registry
            .get_channel_by_decryptor_address(&local_decryptor);
        let handle = match entry {
            None => {
                warn!("No secure channel found for local decryptor {local_decryptor}",);
                return Ok(());
            }
            Some(entry) => {
                let response: SecureChannelApiResponse = context
                    .send_and_receive(
                        route![entry.decryptor_api_address().clone()],
                        SecureChannelApiRequest::ExtractKey,
                    )
                    .await?;

                let key_identifier = match response {
                    SecureChannelApiResponse::Ok(key_identifier) => key_identifier,
                    SecureChannelApiResponse::Err(error) => {
                        error!("Error extracting key: {error}");
                        return Ok(());
                    }
                };

                let secret = self
                    .secure_channel_vault
                    .export_rekey(&key_identifier)
                    .await?;

                self.encryption_at_rest.import_aead_key(secret).await?
            }
        };

        let now = TimestampInSeconds(self.clock.now()?);
        let valid_until = now + self.key_validity;
        let rotate_after = now + self.key_rotation;

        context
            .send(
                message.return_route().clone(),
                KeyExchangeResponse {
                    key_identifier_for_consumer: handle.into_vec(),
                    valid_until,
                    rotate_after,
                    rekey_period: self.rekey_period,
                },
            )
            .await?;

        Ok(())
    }
}

impl KafkaKeyExchangeListener {
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        clock: impl Clock,
        context: &Context,
        encryption_at_rest: Arc<dyn VaultForEncryptionAtRest>,
        secure_channel_vault: Arc<dyn VaultForSecureChannels>,
        secure_channel_registry: SecureChannelRegistry,
        key_rotation: Duration,
        key_validity: Duration,
        rekey_period: Duration,
        incoming_access_control: impl IncomingAccessControl,
        outgoing_access_control: impl OutgoingAccessControl,
    ) -> ockam_core::Result<()> {
        let address = Address::from_string(DefaultAddress::KAFKA_CUSTODIAN);
        let secure_channel_flow_control = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Channel,
                    Kind::NotFound,
                    "Secure channel listener flow control not found",
                )
            })?;

        context
            .flow_controls()
            .add_consumer(address.clone(), &secure_channel_flow_control);

        WorkerBuilder::new(KafkaKeyExchangeListener {
            encryption_at_rest,
            secure_channel_vault,
            key_rotation,
            key_validity,
            rekey_period,
            secure_channel_registry,
            clock: Box::new(clock),
        })
        .with_address(address)
        .with_incoming_access_control(incoming_access_control)
        .with_outgoing_access_control(outgoing_access_control)
        .start(context)
        .await
    }
}
