use crate::kafka::key_exchange::controller::{
    InnerSecureChannelController, KafkaKeyExchangeControllerImpl, RequiredOperation,
    TopicEncryptionKey, TopicEncryptionKeyState,
};
use crate::kafka::ConsumerResolution;
use crate::nodes::service::SecureChannelType;
use crate::DefaultAddress;
use ockam::identity::{SecureChannelRegistryEntry, TimestampInSeconds};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result};
use ockam_multiaddr::proto::{Secure, Service};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use time::Duration;
use tokio::sync::MutexGuard;

impl KafkaKeyExchangeControllerImpl {
    /// Creates a secure channel for the given destination, for key exchange only.
    async fn create_key_exchange_only_secure_channel(
        inner: &MutexGuard<'_, InnerSecureChannelController>,
        context: &Context,
        mut destination: MultiAddr,
    ) -> Result<Address> {
        destination.push_back(Service::new(DefaultAddress::KEY_EXCHANGER_LISTENER))?;
        if let Some(node_manager) = inner.node_manager.upgrade() {
            let secure_channel = node_manager
                .create_secure_channel(
                    context,
                    destination,
                    None,
                    None,
                    None,
                    None,
                    SecureChannelType::KeyExchangeOnly,
                )
                .await?;

            // TODO: temporary workaround until the secure channel persistence is changed
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Ok(secure_channel.encryptor_address().clone())
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
        let encryptor_address;

        let now = TimestampInSeconds(inner.clock.now()?);
        let secure_channels = inner.secure_channels.clone();
        if let Some(encryption_key) = inner.producer_topic_encryptor_map.get_mut(topic_name) {
            // before using it, check if it's still valid
            match encryption_key.operation(now)? {
                RequiredOperation::None => {
                    // the key is still valid
                    rekey_counter = encryption_key.rekey_counter;
                    encryptor_address = encryption_key.producer_encryptor_address.clone();
                }
                RequiredOperation::Rekey => {
                    encryption_key.rekey(context, &secure_channels, now).await?;
                    rekey_counter = encryption_key.rekey_counter;
                    encryptor_address = encryption_key.producer_encryptor_address.clone();
                }
                RequiredOperation::ShouldRotate => {
                    encryption_key.mark_rotation_attempt();
                    // the key is still valid, but it's time to rotate it
                    let result = self.exchange_key(context, topic_name, &mut inner).await;
                    match result {
                        Ok(producer_encryptor_address) => {
                            rekey_counter = 0;
                            encryptor_address = producer_encryptor_address;
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
                                encryption_key.rekey(context, &secure_channels, now).await?;
                            }

                            encryptor_address = encryption_key.producer_encryptor_address.clone();
                            rekey_counter = encryption_key.rekey_counter;
                        }
                    }
                }
                RequiredOperation::MustRotate => {
                    // the key is no longer valid, must not be reused
                    rekey_counter = 0;
                    encryptor_address = self.exchange_key(context, topic_name, &mut inner).await?;
                }
            };
        } else {
            rekey_counter = 0;
            encryptor_address = self.exchange_key(context, topic_name, &mut inner).await?;
        };

        let entry =         secure_channels
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&encryptor_address)
            .ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    format!("cannot find secure channel address `{encryptor_address}` in local registry"),
                )
            })?;

        Ok(TopicEncryptionKey {
            rekey_counter,
            encryptor_api_address: entry.encryptor_api_address().clone(),
            consumer_decryptor_address: entry.their_decryptor_address().clone(),
        })
    }

    async fn exchange_key(
        &self,
        context: &mut Context,
        topic_name: &str,
        inner: &mut MutexGuard<'_, InnerSecureChannelController>,
    ) -> Result<Address> {
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

        let producer_encryptor_address =
            Self::create_key_exchange_only_secure_channel(inner, context, destination.clone())
                .await?;

        if let Some(entry) = inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&producer_encryptor_address)
        {
            if let Err(error) = Self::validate_consumer_credentials(inner, &entry).await {
                if let Some(node_manager) = inner.node_manager.upgrade() {
                    node_manager
                        .delete_secure_channel(context, &producer_encryptor_address)
                        .await?;
                }
                return Err(error);
            };
        } else {
            return Err(Error::new(
                Origin::Transport,
                Kind::Internal,
                format!(
                    "cannot find secure channel address `{producer_encryptor_address}` in local registry"
                ),
            ));
        }

        let now = TimestampInSeconds(inner.clock.now()?);

        // TODO: retrieve these values from the other party
        let valid_until = now + TimestampInSeconds(10 * 60); // 10 minutes
        let rotate_after = now + TimestampInSeconds(5 * 60); // 5 minutes
        let rekey_period = Duration::minutes(1);

        let encryption_key = TopicEncryptionKeyState {
            producer_encryptor_address: producer_encryptor_address.clone(),
            valid_until,
            rotate_after,
            rekey_period,
            last_rekey: now,
            last_rotation_attempt: TimestampInSeconds(0),
            rekey_counter: 0,
        };

        inner
            .producer_topic_encryptor_map
            .insert(topic_name.to_string(), encryption_key);

        info!("Successfully exchanged new key with {destination} for topic {topic_name}");
        Ok(producer_encryptor_address)
    }

    async fn validate_consumer_credentials(
        inner: &MutexGuard<'_, InnerSecureChannelController>,
        entry: &SecureChannelRegistryEntry,
    ) -> Result<()> {
        let authorized = inner
            .consumer_policy_access_control
            .is_identity_authorized(entry.their_id())
            .await?;
        if authorized {
            Ok(())
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!(
                    "unauthorized secure channel for consumer with identifier {}",
                    entry.their_id()
                ),
            ))
        }
    }

    /// Returns the secure channel entry for the consumer decryptor address and validate it
    /// against the producer manual policy.
    pub(crate) async fn get_or_load_secure_channel_decryptor_api_address_for(
        &self,
        ctx: &Context,
        decryptor_remote_address: &Address,
    ) -> Result<Address> {
        let inner = self.inner.lock().await;
        let (decryptor_api_address, their_identifier) = match inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_decryptor_address(decryptor_remote_address)
        {
            Some(entry) => (
                entry.decryptor_api_address().clone(),
                entry.their_id().clone(),
            ),
            None => {
                match inner
                    .secure_channels
                    .start_persisted_secure_channel_decryptor(ctx, decryptor_remote_address)
                    .await
                {
                    Ok(sc) => (
                        sc.decryptor_api_address().clone(),
                        sc.their_identifier().clone(),
                    ),
                    Err(e) => {
                        return Err(Error::new(
                            Origin::Channel,
                            Kind::Unknown,
                            format!(
                                "secure channel decryptor {} can not be retrieved: {e:?}",
                                decryptor_remote_address.address()
                            ),
                        ));
                    }
                }
            }
        };

        let authorized = inner
            .producer_policy_access_control
            .is_identity_authorized(&their_identifier)
            .await?;

        if authorized {
            Ok(decryptor_api_address)
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!(
                    "unauthorized secure channel for producer with identifier {}",
                    their_identifier
                ),
            ))
        }
    }
}
