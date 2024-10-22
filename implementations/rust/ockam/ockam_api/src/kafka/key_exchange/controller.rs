use crate::kafka::key_exchange::KafkaKeyExchangeController;
use crate::kafka::protocol_aware::KafkaEncryptedContent;
use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::nodes::NodeManager;
use ockam::identity::{
    utils, DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
    SecureChannels, TimestampInSeconds,
};
use ockam_abac::PolicyAccessControl;
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, Address, Error};
use ockam_node::Context;
use std::sync::{Arc, Weak};
use time::Duration;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct KafkaKeyExchangeControllerImpl {
    pub(crate) inner: Arc<Mutex<InnerSecureChannelController>>,
}

#[async_trait]
impl KafkaKeyExchangeController for KafkaKeyExchangeControllerImpl {
    async fn encrypt_content(
        &self,
        context: &mut Context,
        topic_name: &str,
        content: Vec<u8>,
    ) -> ockam_core::Result<KafkaEncryptedContent> {
        let topic_key_handler = self.get_or_exchange_key(context, topic_name).await?;
        let encryption_response: EncryptionResponse = context
            .send_and_receive(
                route![topic_key_handler.encryptor_api_address.clone()],
                EncryptionRequest::Encrypt(content),
            )
            .await?;

        let encrypted_content = match encryption_response {
            EncryptionResponse::Ok(p) => p,
            EncryptionResponse::Err(cause) => {
                warn!("Cannot encrypt kafka message");
                return Err(cause);
            }
        };

        Ok(KafkaEncryptedContent {
            content: encrypted_content,
            consumer_decryptor_address: topic_key_handler.consumer_decryptor_address,
            rekey_counter: topic_key_handler.rekey_counter,
        })
    }

    async fn decrypt_content(
        &self,
        context: &mut Context,
        consumer_decryptor_address: &Address,
        rekey_counter: u16,
        encrypted_content: Vec<u8>,
    ) -> ockam_core::Result<Vec<u8>> {
        let secure_channel_decryptor_api_address = self
            .get_or_load_secure_channel_decryptor_api_address_for(
                context,
                consumer_decryptor_address,
            )
            .await?;

        let decrypt_response = context
            .send_and_receive(
                route![secure_channel_decryptor_api_address],
                DecryptionRequest(encrypted_content, Some(rekey_counter)),
            )
            .await?;

        let decrypted_content = match decrypt_response {
            DecryptionResponse::Ok(p) => p,
            DecryptionResponse::Err(cause) => {
                error!("cannot decrypt kafka message: closing connection");
                return Err(cause);
            }
        };

        Ok(decrypted_content)
    }

    async fn publish_consumer(
        &self,
        context: &mut Context,
        topic_name: &str,
    ) -> ockam_core::Result<()> {
        let mut inner = self.inner.lock().await;

        match inner.consumer_publishing.clone() {
            ConsumerPublishing::None => {}
            ConsumerPublishing::Relay(where_to_publish) => {
                if inner.topic_relay_set.contains(topic_name) {
                    return Ok(());
                }
                let alias = format!("consumer_{topic_name}");

                if let Some(node_manager) = inner.node_manager.upgrade() {
                    let relay_info = node_manager
                        .create_relay(
                            context,
                            &where_to_publish.clone(),
                            alias.clone(),
                            None,
                            Some(alias),
                        )
                        .await?;
                    trace!("remote relay created: {relay_info:?}");
                    inner.topic_relay_set.insert(topic_name.to_string());
                }
            }
        }

        Ok(())
    }
}

pub(crate) struct TopicEncryptionKeyHandler {
    pub(crate) rekey_counter: u16,
    pub(crate) encryptor_api_address: Address,
    pub(crate) consumer_decryptor_address: Address,
}

const ROTATION_RETRY_DELAY: Duration = Duration::minutes(5);
pub(crate) struct TopicEncryptionKey {
    pub(crate) producer_encryptor_address: Address,
    pub(crate) valid_until: TimestampInSeconds,
    pub(crate) rotate_after: TimestampInSeconds,
    pub(crate) last_rekey: TimestampInSeconds,
    pub(crate) rekey_counter: u16,
    pub(crate) rekey_period: Duration,
    pub(crate) last_rotation_attempt: TimestampInSeconds,
}

pub(crate) enum RequiredOperation {
    Rekey,
    ShouldRotate,
    MustRotate,
    None,
}

impl TopicEncryptionKey {
    /// Return the operation that should be performed on the key before using it
    pub(crate) fn operation(&self) -> ockam_core::Result<RequiredOperation> {
        let now = utils::now()?;
        if now >= self.valid_until {
            return Ok(RequiredOperation::MustRotate);
        }

        if now >= self.rotate_after
            && now >= self.last_rotation_attempt + ROTATION_RETRY_DELAY.whole_seconds() as u64
        {
            return Ok(RequiredOperation::ShouldRotate);
        }

        if now >= self.last_rekey + self.rekey_period.whole_seconds() as u64 {
            return Ok(RequiredOperation::Rekey);
        }

        Ok(RequiredOperation::None)
    }

    pub(crate) fn mark_rotation_attempt(&mut self) {
        self.last_rotation_attempt = utils::now().unwrap();
    }

    pub(crate) async fn rekey(
        &mut self,
        context: &mut Context,
        secure_channel: &SecureChannels,
    ) -> ockam_core::Result<()> {
        if self.rekey_counter == u16::MAX {
            return Err(Error::new(
                Origin::Channel,
                Kind::Unknown,
                "Rekey counter overflow",
            ));
        }

        let encryptor_address = &self.producer_encryptor_address;

        let secure_channel_entry = secure_channel.secure_channel_registry().get_channel_by_encryptor_address(
            encryptor_address,
        ).ok_or_else(|| {
            Error::new(
                Origin::Channel,
                Kind::Unknown,
                format!("Cannot find secure channel address `{encryptor_address}` in local registry"),
            )
        })?;

        let rekey_response: EncryptionResponse = context
            .send_and_receive(
                route![secure_channel_entry.encryptor_api_address().clone()],
                EncryptionRequest::Rekey,
            )
            .await?;

        match rekey_response {
            EncryptionResponse::Ok(_) => {}
            EncryptionResponse::Err(cause) => {
                error!("Cannot rekey secure channel: {cause}");
                return Err(cause);
            }
        }

        self.last_rekey = utils::now()?;
        self.rekey_counter += 1;

        Ok(())
    }
}

pub(crate) type TopicName = String;

pub struct InnerSecureChannelController {
    // we identify the secure channel instance by using the decryptor address of the consumer
    // which is known to both parties
    pub(crate) producer_topic_encryptor_map: HashMap<TopicName, TopicEncryptionKey>,
    pub(crate) node_manager: Weak<NodeManager>,
    // describes how to reach the consumer node
    pub(crate) consumer_resolution: ConsumerResolution,
    // describes if/how to publish the consumer
    pub(crate) consumer_publishing: ConsumerPublishing,
    pub(crate) topic_relay_set: HashSet<String>,
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) consumer_policy_access_control: PolicyAccessControl,
    pub(crate) producer_policy_access_control: PolicyAccessControl,
}

impl KafkaKeyExchangeControllerImpl {
    pub(crate) fn new(
        node_manager: Arc<NodeManager>,
        secure_channels: Arc<SecureChannels>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        consumer_policy_access_control: PolicyAccessControl,
        producer_policy_access_control: PolicyAccessControl,
    ) -> KafkaKeyExchangeControllerImpl {
        Self {
            inner: Arc::new(Mutex::new(InnerSecureChannelController {
                producer_topic_encryptor_map: Default::default(),
                topic_relay_set: Default::default(),
                node_manager: Arc::downgrade(&node_manager),
                secure_channels,
                consumer_resolution,
                consumer_publishing,
                consumer_policy_access_control,
                producer_policy_access_control,
            })),
        }
    }
}
