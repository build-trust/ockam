use crate::kafka::key_exchange::{KafkaKeyExchangeController, TopicPartition};
use crate::kafka::protocol_aware::KafkaEncryptedContent;
use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::nodes::models::relay::ReturnTiming;
use crate::nodes::NodeManager;
use ockam::identity::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse, Identifier,
    SecureChannels,
};
use ockam_abac::PolicyAccessControl;
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::{async_trait, route, Address};
use ockam_node::Context;
use std::sync::Arc;
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
        partition_index: i32,
        content: Vec<u8>,
    ) -> ockam_core::Result<KafkaEncryptedContent> {
        let secure_channel_entry = self
            .get_or_create_secure_channel(context, topic_name, partition_index)
            .await?;

        let consumer_decryptor_address = secure_channel_entry.their_decryptor_address();

        trace!("encrypting content with {consumer_decryptor_address}");
        let encryption_response: EncryptionResponse = context
            .send_and_receive(
                route![secure_channel_entry.encryptor_api_address().clone()],
                EncryptionRequest(content),
            )
            .await?;

        let encrypted_content = match encryption_response {
            EncryptionResponse::Ok(p) => p,
            EncryptionResponse::Err(cause) => {
                warn!("cannot encrypt kafka message");
                return Err(cause);
            }
        };

        trace!("encrypted content with {consumer_decryptor_address}");
        Ok(KafkaEncryptedContent {
            content: encrypted_content,
            consumer_decryptor_address,
        })
    }

    async fn decrypt_content(
        &self,
        context: &mut Context,
        consumer_decryptor_address: &Address,
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
                DecryptionRequest(encrypted_content),
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
        partitions: Vec<i32>,
    ) -> ockam_core::Result<()> {
        let mut inner = self.inner.lock().await;

        match inner.consumer_publishing.clone() {
            ConsumerPublishing::None => {}
            ConsumerPublishing::Relay(where_to_publish) => {
                for partition in partitions {
                    let topic_key: TopicPartition = (topic_name.to_string(), partition);
                    if inner.topic_relay_set.contains(&topic_key) {
                        continue;
                    }
                    let alias = format!("consumer_{topic_name}_{partition}");
                    let relay_info = inner
                        .node_manager
                        .create_relay(
                            context,
                            &where_to_publish.clone(),
                            alias.clone(),
                            None,
                            Some(alias),
                            ReturnTiming::AfterConnection,
                        )
                        .await?;

                    trace!("remote relay created: {relay_info:?}");
                    inner.topic_relay_set.insert(topic_key);
                }
            }
        }

        Ok(())
    }
}

pub struct InnerSecureChannelController {
    // we identify the secure channel instance by using the decryptor address of the consumer
    // which is known to both parties
    pub(crate) topic_encryptor_map: HashMap<TopicPartition, Address>,
    // since topic/partition is using a key exchange only secure channel,
    // we need another secure channel for each consumer identifier
    // to make sure the relative credential is properly updated
    pub(crate) identity_encryptor_map: HashMap<Identifier, Address>,
    pub(crate) node_manager: Arc<NodeManager>,
    // describes how to reach the consumer node
    pub(crate) consumer_resolution: ConsumerResolution,
    // describes if/how to publish the consumer
    pub(crate) consumer_publishing: ConsumerPublishing,
    pub(crate) topic_relay_set: HashSet<TopicPartition>,
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
                topic_encryptor_map: Default::default(),
                identity_encryptor_map: Default::default(),
                topic_relay_set: Default::default(),
                node_manager,
                secure_channels,
                consumer_resolution,
                consumer_publishing,
                consumer_policy_access_control,
                producer_policy_access_control,
            })),
        }
    }
}
