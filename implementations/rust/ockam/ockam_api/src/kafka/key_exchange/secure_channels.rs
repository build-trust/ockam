use crate::kafka::key_exchange::controller::{
    InnerSecureChannelController, KafkaKeyExchangeControllerImpl,
};
use crate::kafka::ConsumerResolution;
use crate::nodes::service::SecureChannelType;
use crate::DefaultAddress;
use ockam::identity::SecureChannelRegistryEntry;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result};
use ockam_multiaddr::proto::{Secure, Service};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use tokio::sync::MutexGuard;

impl KafkaKeyExchangeControllerImpl {
    /// Creates a secure channel for the given destination.
    async fn create_secure_channel(
        inner: &MutexGuard<'_, InnerSecureChannelController>,
        context: &Context,
        mut destination: MultiAddr,
    ) -> Result<Address> {
        destination.push_back(Service::new(DefaultAddress::SECURE_CHANNEL_LISTENER))?;
        let secure_channel = inner
            .node_manager
            .create_secure_channel(
                context,
                destination,
                None,
                None,
                None,
                None,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;
        Ok(secure_channel.encryptor_address().clone())
    }

    /// Creates a secure channel for the given destination, for key exchange only.
    async fn create_key_exchange_only_secure_channel(
        inner: &MutexGuard<'_, InnerSecureChannelController>,
        context: &Context,
        mut destination: MultiAddr,
    ) -> Result<Address> {
        destination.push_back(Service::new(DefaultAddress::KEY_EXCHANGER_LISTENER))?;
        let secure_channel = inner
            .node_manager
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
        Ok(secure_channel.encryptor_address().clone())
    }

    /// Creates a secure channel from the producer to the consumer needed to encrypt messages.
    /// Returns the relative secure channel entry.
    pub(crate) async fn get_or_create_secure_channel(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition: i32,
    ) -> Result<SecureChannelRegistryEntry> {
        let mut inner = self.inner.lock().await;

        // TODO: it may be better to exchange a new key for each partition
        // when we have only one consumer, we use the same secure channel for all topics
        let topic_partition_key = match &inner.consumer_resolution {
            ConsumerResolution::SingleNode(_) | ConsumerResolution::None => ("".to_string(), 0i32),
            ConsumerResolution::ViaRelay(_) => (topic_name.to_string(), partition),
        };

        let encryptor_address = {
            if let Some(encryptor_address) = inner.topic_encryptor_map.get(&topic_partition_key) {
                encryptor_address.clone()
            } else {
                // destination is without the final service
                let destination = match inner.consumer_resolution.clone() {
                    ConsumerResolution::SingleNode(mut destination) => {
                        debug!("creating new direct secure channel to consumer: {destination}");
                        // remove /secure/api service from the destination if present
                        if let Some(service) = destination.last() {
                            let service: Option<Secure> = service.cast();
                            if let Some(service) = service {
                                if service.as_bytes()
                                    == DefaultAddress::SECURE_CHANNEL_LISTENER.as_bytes()
                                {
                                    destination.pop_back();
                                }
                            }
                        }
                        destination
                    }
                    ConsumerResolution::ViaRelay(mut destination) => {
                        // consumer_ is the arbitrary chosen prefix by both parties
                        let topic_partition_address =
                            format!("forward_to_consumer_{topic_name}_{partition}");
                        debug!(
                            "creating new secure channel via relay to {topic_partition_address}"
                        );
                        destination.push_back(Service::new(topic_partition_address))?;
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

                let producer_encryptor_address = Self::create_key_exchange_only_secure_channel(
                    &inner,
                    context,
                    destination.clone(),
                )
                .await?;

                if let Some(entry) = inner
                    .secure_channels
                    .secure_channel_registry()
                    .get_channel_by_encryptor_address(&producer_encryptor_address)
                {
                    if let Err(error) = Self::validate_consumer_credentials(&inner, &entry).await {
                        inner
                            .node_manager
                            .delete_secure_channel(context, &producer_encryptor_address)?;
                        return Err(error);
                    };

                    // creates a dedicated secure channel to the consumer to keep the
                    // credentials up to date
                    if !inner.identity_encryptor_map.contains_key(entry.their_id()) {
                        if let Err(err) =
                            Self::create_secure_channel(&inner, context, destination).await
                        {
                            inner
                                .node_manager
                                .delete_secure_channel(context, &producer_encryptor_address)?;
                            return Err(err);
                        }
                    }
                } else {
                    return Err(Error::new(
                        Origin::Transport,
                        Kind::Internal,
                        format!(
                            "cannot find secure channel address `{producer_encryptor_address}` in local registry"
                        ),
                    ));
                }

                inner
                    .topic_encryptor_map
                    .insert(topic_partition_key, producer_encryptor_address.clone());

                debug!("created secure channel");
                producer_encryptor_address
            }
        };

        inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&encryptor_address)
            .ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    format!("cannot find secure channel address `{encryptor_address}` in local registry"),
                )
            })
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
