use crate::kafka::key_exchange::controller::{
    InnerSecureChannelController, KafkaKeyExchangeControllerImpl, RequiredOperation,
    TopicEncryptionKey, TopicEncryptionKeyState,
};
use crate::kafka::key_exchange::listener::{KeyExchangeRequest, KeyExchangeResponse};
use crate::kafka::ConsumerResolution;
use crate::DefaultAddress;
use ockam::identity::{SecureChannelApiRequest, SecureChannelApiResponse, TimestampInSeconds};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error, Result};
use ockam_multiaddr::proto::{Secure, Service};
use ockam_multiaddr::MultiAddr;
use ockam_node::{Context, MessageSendReceiveOptions};
use ockam_vault::AeadSecretKeyHandle;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::MutexGuard;

pub(crate) struct ExchangedKey {
    pub secret_key_handler: AeadSecretKeyHandle,
    pub key_identifier_for_consumer: Vec<u8>,
    pub valid_until: TimestampInSeconds,
    pub rotate_after: TimestampInSeconds,
    pub rekey_period: Duration,
}

impl KafkaKeyExchangeControllerImpl {
    /// Creates a secure channel for the given destination, for key exchange only.
    async fn ask_for_key_over_secure_channel(
        &self,
        inner: &MutexGuard<'_, InnerSecureChannelController>,
        context: &Context,
        mut destination: MultiAddr,
    ) -> Result<ExchangedKey> {
        destination.push_back(Secure::new(DefaultAddress::SECURE_CHANNEL_LISTENER))?;
        destination.push_back(Service::new(DefaultAddress::KAFKA_CUSTODIAN))?;
        if let Some(node_manager) = inner.node_manager.upgrade() {
            // create a second secure channel to be used for key exchange
            let (aead_secret_key_handle, their_decryptor_address) = {
                let key_exchange_connection = node_manager
                    .make_connection(context, &destination, node_manager.identifier(), None, None)
                    .await?;

                let encryptor_address = key_exchange_connection
                    .secure_channel_encryptors
                    .first()
                    .expect("encryptor should be present");

                let entry = node_manager
                    .secure_channels
                    .secure_channel_registry()
                    .get_channel_by_encryptor_address(encryptor_address)
                    .expect("channel should be present");

                if !inner
                    .consumer_policy_access_control
                    .is_identity_authorized(entry.their_id())
                    .await?
                {
                    key_exchange_connection
                        .close(context, &node_manager)
                        .await?;
                    return Err(Error::new(
                        Origin::Channel,
                        Kind::Invalid,
                        "Consumer is not authorized to use the secure channel",
                    ));
                }

                let response: SecureChannelApiResponse = context
                    .send_and_receive(
                        route![entry.encryptor_api_address().clone()],
                        SecureChannelApiRequest::ExtractKey,
                    )
                    .await?;

                match response {
                    SecureChannelApiResponse::Ok(secret_handle) => {
                        let secret = node_manager
                            .secure_channels
                            .vault()
                            .secure_channel_vault
                            .export_rekey(&secret_handle)
                            .await?;

                        key_exchange_connection
                            .close(context, &node_manager)
                            .await?;

                        (
                            self.encryption_at_rest.import_aead_key(secret).await?,
                            entry.their_decryptor_address(),
                        )
                    }
                    SecureChannelApiResponse::Err(error) => {
                        key_exchange_connection
                            .close(context, &node_manager)
                            .await?;
                        return Err(error);
                    }
                }
            };

            let connection = node_manager
                .make_connection(context, &destination, node_manager.identifier(), None, None)
                .await?;

            let send_and_receive_options = MessageSendReceiveOptions::new()
                .with_incoming_access_control(Arc::new(
                    inner.consumer_policy_access_control.create_incoming(),
                ))
                .with_outgoing_access_control(Arc::new(
                    inner
                        .consumer_policy_access_control
                        .create_outgoing(context)
                        .await?,
                ));

            let route = connection.route()?;
            let response: KeyExchangeResponse = context
                .send_and_receive_extended(
                    route,
                    KeyExchangeRequest {
                        local_decryptor_address: their_decryptor_address,
                    },
                    send_and_receive_options,
                )
                .await?
                .into_body()?;

            connection.close(context, &node_manager).await?;

            Ok(ExchangedKey {
                secret_key_handler: aead_secret_key_handle,
                key_identifier_for_consumer: response.key_identifier_for_consumer,
                valid_until: response.valid_until,
                rotate_after: response.rotate_after,
                rekey_period: response.rekey_period,
            })
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::Internal,
                "Node Manager is not available",
            ))
        }
    }

    /// Creates a secure channel from the producer to the consumer needed to encrypt messages.
    /// Returns the relative encryption key information.
    pub(crate) async fn get_or_exchange_key(
        &self,
        context: &mut Context,
        topic_name: &str,
    ) -> Result<TopicEncryptionKey> {
        let mut inner = self.inner.lock().await;

        let rekey_counter;
        let secret_key_handle;
        let key_identifier_for_consumer;

        let now = TimestampInSeconds(inner.clock.now()?);
        if let Some(encryption_key) = inner.producer_topic_encryptor_map.get_mut(topic_name) {
            // before using it, check if it's still valid
            match encryption_key.operation(now)? {
                RequiredOperation::None => {
                    // the key is still valid
                    rekey_counter = encryption_key.rekey_counter;
                    secret_key_handle = encryption_key.secret_key_handle.clone();
                    key_identifier_for_consumer =
                        encryption_key.key_identifier_for_consumer.clone();
                }
                RequiredOperation::Rekey => {
                    encryption_key.rekey(&self.encryption_at_rest, now).await?;
                    rekey_counter = encryption_key.rekey_counter;
                    secret_key_handle = encryption_key.secret_key_handle.clone();
                    key_identifier_for_consumer =
                        encryption_key.key_identifier_for_consumer.clone();
                }
                RequiredOperation::ShouldRotate => {
                    encryption_key.mark_rotation_attempt();
                    // the key is still valid, but it's time to rotate it
                    let result = self.exchange_key(context, topic_name, &mut inner).await;
                    match result {
                        Ok(new_state) => {
                            rekey_counter = new_state.rekey_counter;
                            secret_key_handle = new_state.secret_key_handle.clone();
                            key_identifier_for_consumer =
                                new_state.key_identifier_for_consumer.clone();
                        }
                        Err(error) => {
                            warn!(
                                "Failed to rotate encryption key for topic `{topic_name}`: {error}. The current key will be used instead."
                            );

                            // borrow it again to satisfy the borrow checker
                            let encryption_key = inner
                                .producer_topic_encryptor_map
                                .get_mut(topic_name)
                                .expect("key should be present");

                            // we might still need to rekey
                            if let RequiredOperation::Rekey = encryption_key.operation(now)? {
                                encryption_key.rekey(&self.encryption_at_rest, now).await?;
                            }

                            secret_key_handle = encryption_key.secret_key_handle.clone();
                            rekey_counter = encryption_key.rekey_counter;
                            key_identifier_for_consumer =
                                encryption_key.key_identifier_for_consumer.clone();
                        }
                    }
                }
                RequiredOperation::MustRotate => {
                    // the key is no longer valid, must not be reused
                    let new_key_state = self.exchange_key(context, topic_name, &mut inner).await?;
                    secret_key_handle = new_key_state.secret_key_handle.clone();
                    key_identifier_for_consumer = new_key_state.key_identifier_for_consumer.clone();
                    rekey_counter = new_key_state.rekey_counter;
                }
            };
        } else {
            let new_key_state = self.exchange_key(context, topic_name, &mut inner).await?;
            secret_key_handle = new_key_state.secret_key_handle.clone();
            key_identifier_for_consumer = new_key_state.key_identifier_for_consumer.clone();
            rekey_counter = new_key_state.rekey_counter;
        };

        Ok(TopicEncryptionKey {
            rekey_counter,
            secret_key_handle,
            key_identifier_for_consumer,
        })
    }

    async fn exchange_key<'a>(
        &'a self,
        context: &mut Context,
        topic_name: &str,
        inner: &'a mut MutexGuard<'_, InnerSecureChannelController>,
    ) -> Result<&'a TopicEncryptionKeyState> {
        // destination is without the final service
        let destination = match inner.consumer_resolution.clone() {
            ConsumerResolution::SingleNode(mut destination) => {
                debug!("creating new direct secure channel to consumer: {destination}");
                // remove /secure/api service from the destination if present
                if let Some(service) = destination.last() {
                    let service: Option<Secure> = service.cast();
                    if let Some(service) = service {
                        if service.as_bytes() == DefaultAddress::SECURE_CHANNEL_LISTENER.as_bytes()
                        {
                            destination.pop_back();
                        }
                    }
                }
                destination
            }
            ConsumerResolution::ViaRelay(mut destination) => {
                // consumer_ is the arbitrary chosen prefix by both parties
                let topic_address = format!("forward_to_consumer_{topic_name}");
                debug!("creating new secure channel via relay to {topic_address}");
                destination.push_back(Service::new(topic_address))?;
                destination
            }
            ConsumerResolution::None => {
                return Err(Error::new(
                    Origin::Transport,
                    Kind::Invalid,
                    "cannot encrypt messages with consumer key when consumer route resolution is not set",
                ));
            }
        };

        let exchanged_key = self
            .ask_for_key_over_secure_channel(inner, context, destination.clone())
            .await?;

        let now = TimestampInSeconds(inner.clock.now()?);
        let encryption_key = TopicEncryptionKeyState {
            secret_key_handle: exchanged_key.secret_key_handler,
            key_identifier_for_consumer: exchanged_key.key_identifier_for_consumer,
            valid_until: exchanged_key.valid_until,
            rotate_after: exchanged_key.rotate_after,
            rekey_period: exchanged_key.rekey_period,
            last_rekey: now,
            last_rotation_attempt: TimestampInSeconds(0),
            rekey_counter: 0,
        };

        inner
            .producer_topic_encryptor_map
            .insert(topic_name.to_string(), encryption_key);

        let encryption_key = inner
            .producer_topic_encryptor_map
            .get(topic_name)
            .expect("key should be present");

        info!("Successfully exchanged new key with {destination} for topic {topic_name}");
        debug!("Exchanged key: {encryption_key:?}");
        Ok(encryption_key)
    }
}
