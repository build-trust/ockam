use crate::kafka::key_exchange::KafkaKeyExchangeController;
use crate::kafka::protocol_aware::KafkaEncryptedContent;
use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::nodes::models::relay::ReturnTiming;
use crate::nodes::NodeManager;
use ockam::identity::{utils, TimestampInSeconds, AES_GCM_NONCE_LEN};
use ockam_abac::PolicyAccessControl;
use ockam_core::compat::clock::{Clock, ProductionClock};
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error};
use ockam_node::Context;
use ockam_vault::{AeadSecretKeyHandle, HandleToSecret, VaultForEncryptionAtRest};
use std::sync::{Arc, Weak};
use time::Duration;
use tokio::sync::Mutex;

const AES_GCM_TAGSIZE: usize = 16;

#[derive(Clone)]
pub(crate) struct KafkaKeyExchangeControllerImpl {
    pub(crate) inner: Arc<Mutex<InnerSecureChannelController>>,
    // outside, so we don't need to lock the inner when performing encryption/decryption
    pub(crate) encryption_at_rest: Arc<dyn VaultForEncryptionAtRest>,
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

        let content_str = String::from_utf8_lossy(&content);
        debug!("Encrypting content for topic: {topic_name}, content: {content_str}");

        let mut destination = vec![0; AES_GCM_NONCE_LEN + content.len() + AES_GCM_TAGSIZE];
        destination[AES_GCM_NONCE_LEN..content.len() + AES_GCM_NONCE_LEN].copy_from_slice(&content);

        let content_str = String::from_utf8_lossy(&destination);
        debug!("Destination: {destination:? }, content: {content_str}");

        self.encryption_at_rest
            .aead_encrypt(&topic_key_handler.secret_key_handle, &mut destination, &[])
            .await?;

        Ok(KafkaEncryptedContent {
            content: destination,
            secret_key_handler: topic_key_handler.key_identifier_for_consumer,
            rekey_counter: topic_key_handler.rekey_counter,
        })
    }

    async fn decrypt_content<'a>(
        &self,
        kafka_encrypted_content: &'a mut KafkaEncryptedContent,
    ) -> ockam_core::Result<&'a [u8]> {
        let secret_key_handle = AeadSecretKeyHandle::new(HandleToSecret::new(
            kafka_encrypted_content.secret_key_handler.clone(),
        ));

        Ok(self
            .encryption_at_rest
            .aead_decrypt(
                &secret_key_handle,
                kafka_encrypted_content.rekey_counter,
                &mut kafka_encrypted_content.content,
                &[],
            )
            .await?)
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
                            ReturnTiming::AfterConnection,
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

#[derive(Debug, PartialEq)]
pub(crate) struct TopicEncryptionKey {
    pub(crate) rekey_counter: u16,
    pub(crate) secret_key_handle: AeadSecretKeyHandle,
    pub(crate) key_identifier_for_consumer: Vec<u8>,
}

const ROTATION_RETRY_DELAY: Duration = Duration::minutes(5);

#[derive(Debug)]
pub(crate) struct TopicEncryptionKeyState {
    pub(crate) secret_key_handle: AeadSecretKeyHandle,
    pub(crate) key_identifier_for_consumer: Vec<u8>,
    pub(crate) valid_until: TimestampInSeconds,
    pub(crate) rotate_after: TimestampInSeconds,
    pub(crate) last_rekey: TimestampInSeconds,
    pub(crate) rekey_counter: u16,
    pub(crate) rekey_period: std::time::Duration,
    pub(crate) last_rotation_attempt: TimestampInSeconds,
}

pub(crate) enum RequiredOperation {
    Rekey,
    ShouldRotate,
    MustRotate,
    None,
}

impl TopicEncryptionKeyState {
    /// Return the operation that should be performed on the key before using it
    pub(crate) fn operation(
        &self,
        now: TimestampInSeconds,
    ) -> ockam_core::Result<RequiredOperation> {
        if now >= self.valid_until {
            return Ok(RequiredOperation::MustRotate);
        }

        if now >= self.rotate_after
            && now >= self.last_rotation_attempt + ROTATION_RETRY_DELAY.whole_seconds() as u64
        {
            return Ok(RequiredOperation::ShouldRotate);
        }

        if now >= self.last_rekey + self.rekey_period.as_secs() {
            return Ok(RequiredOperation::Rekey);
        }

        Ok(RequiredOperation::None)
    }

    pub(crate) fn mark_rotation_attempt(&mut self) {
        self.last_rotation_attempt = utils::now().unwrap();
    }

    pub(crate) async fn rekey(
        &mut self,
        encryption_at_rest_vault: &Arc<dyn VaultForEncryptionAtRest>,
        now: TimestampInSeconds,
    ) -> ockam_core::Result<()> {
        if self.rekey_counter == u16::MAX {
            return Err(Error::new(
                Origin::Channel,
                Kind::Unknown,
                "Rekey counter overflow",
            ));
        }

        self.secret_key_handle = encryption_at_rest_vault
            .rekey_and_delete(&self.secret_key_handle)
            .await?;

        self.last_rekey = now;
        self.rekey_counter += 1;

        Ok(())
    }
}

pub(crate) type TopicName = String;

pub struct InnerSecureChannelController {
    // we identify the secure channel instance by using the decryptor address of the consumer
    // which is known to both parties
    pub(crate) producer_topic_encryptor_map: HashMap<TopicName, TopicEncryptionKeyState>,
    // describes how to reach the consumer node
    pub(crate) consumer_resolution: ConsumerResolution,
    // describes if/how to publish the consumer
    pub(crate) consumer_publishing: ConsumerPublishing,
    pub(crate) topic_relay_set: HashSet<String>,
    pub(crate) consumer_policy_access_control: PolicyAccessControl,
    pub(crate) clock: Box<dyn Clock>,
    pub(crate) node_manager: Weak<NodeManager>,
}

impl KafkaKeyExchangeControllerImpl {
    pub(crate) fn new(
        node_manager: Arc<NodeManager>,
        encryption_at_rest: Arc<dyn VaultForEncryptionAtRest>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        consumer_policy_access_control: PolicyAccessControl,
    ) -> KafkaKeyExchangeControllerImpl {
        Self::new_extended(
            ProductionClock,
            node_manager,
            encryption_at_rest,
            consumer_resolution,
            consumer_publishing,
            consumer_policy_access_control,
        )
    }

    pub(crate) fn new_extended(
        clock: impl Clock,
        node_manager: Arc<NodeManager>,
        encryption_at_rest: Arc<dyn VaultForEncryptionAtRest>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        consumer_policy_access_control: PolicyAccessControl,
    ) -> KafkaKeyExchangeControllerImpl {
        Self {
            encryption_at_rest,
            inner: Arc::new(Mutex::new(InnerSecureChannelController {
                clock: Box::new(clock),
                producer_topic_encryptor_map: Default::default(),
                topic_relay_set: Default::default(),
                node_manager: Arc::downgrade(&node_manager),
                consumer_resolution,
                consumer_publishing,
                consumer_policy_access_control,
            })),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
    use crate::kafka::key_exchange::listener::KafkaKeyExchangeListener;
    use crate::kafka::{ConsumerPublishing, ConsumerResolution};
    use crate::test_utils::{AuthorityConfiguration, TestNode};
    use ockam::identity::Identifier;
    use ockam_abac::{Action, Env, Resource, ResourceType};
    use ockam_core::compat::clock::test::TestClock;
    use ockam_core::AllowAll;
    use ockam_multiaddr::MultiAddr;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::runtime::Runtime;
    use tokio::time::timeout;

    #[test]
    pub fn rekey_rotation() -> ockam_core::Result<()> {
        let runtime = Arc::new(Runtime::new().unwrap());
        let runtime_cloned = runtime.clone();

        runtime_cloned.block_on(async move {
            let test_body = async move {
                TestNode::clean().await?;
                let authority = TestNode::create_extended(
                    runtime.clone(),
                    None,
                    AuthorityConfiguration::SelfReferencing,
                )
                .await;

                let mut consumer_node = TestNode::create_extended(
                    runtime.clone(),
                    None,
                    AuthorityConfiguration::Node(&authority),
                )
                .await;
                let mut producer_node = TestNode::create_extended(
                    runtime.clone(),
                    None,
                    AuthorityConfiguration::Node(&authority),
                )
                .await;

                let test_clock = TestClock::new(0);

                KafkaKeyExchangeListener::create(
                    test_clock.clone(),
                    &consumer_node.context,
                    consumer_node
                        .node_manager
                        .secure_channels
                        .vault()
                        .encryption_at_rest_vault,
                    consumer_node
                        .node_manager
                        .secure_channels
                        .vault()
                        .secure_channel_vault,
                    consumer_node
                        .node_manager
                        .secure_channels
                        .secure_channel_registry(),
                    Duration::from_secs(5 * 60),  //rotation
                    Duration::from_secs(10 * 60), //validity
                    Duration::from_secs(60),
                    AllowAll,
                    AllowAll,
                )
                .await?;

                let destination = consumer_node.listen_address().await.multi_addr().unwrap();
                let producer_secure_channel_controller = create_secure_channel_controller(
                    test_clock.clone(),
                    &mut producer_node,
                    destination.clone(),
                    authority.node_manager.identifier(),
                )
                .await;

                let _consumer_secure_channel_controller = create_secure_channel_controller(
                    test_clock.clone(),
                    &mut consumer_node,
                    destination.clone(),
                    authority.node_manager.identifier(),
                )
                .await;

                let first_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(first_key.rekey_counter, 0);

                // 00:10 - nothing should change
                test_clock.add_seconds(10);

                let second_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(first_key, second_key);

                // 01:00 - the default rekeying period is 1 minute
                test_clock.add_seconds(50);

                let third_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(third_key.rekey_counter, 1);
                assert_eq!(
                    first_key.key_identifier_for_consumer,
                    third_key.key_identifier_for_consumer
                );

                // 04:00 - yet another rekey should happen, but no rotation
                test_clock.add_seconds(60 * 3);

                let fourth_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(fourth_key.rekey_counter, 2);
                assert_eq!(
                    first_key.key_identifier_for_consumer,
                    fourth_key.key_identifier_for_consumer
                );

                // 05:00 - the default duration of the key is 10 minutes,
                // but the rotation should happen after 5 minutes
                test_clock.add_seconds(60);

                let fifth_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_ne!(
                    third_key.key_identifier_for_consumer,
                    fifth_key.key_identifier_for_consumer
                );
                assert_eq!(fifth_key.rekey_counter, 0);

                // Now let's simulate a failure to rekey by shutting down the consumer
                consumer_node.context.stop().await?;
                drop(consumer_node);

                // 06:00 - The producer should still be able to rekey
                test_clock.add_seconds(60);
                let sixth_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(sixth_key.rekey_counter, 1);
                assert_eq!(
                    fifth_key.key_identifier_for_consumer,
                    sixth_key.key_identifier_for_consumer
                );

                // 10:00 - Rotation fails, but the existing key is still valid
                // and needs to be rekeyed
                // (since we exchanged key at 05:00, it should be valid until 15:00)
                test_clock.add_seconds(60 * 4);
                let seventh_key = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await?;

                assert_eq!(seventh_key.rekey_counter, 2);
                assert_eq!(
                    fifth_key.key_identifier_for_consumer,
                    seventh_key.key_identifier_for_consumer
                );

                // 15:00 - Rotation fails, and the existing key is no longer valid
                test_clock.add_seconds(60 * 5);
                let result = producer_secure_channel_controller
                    .get_or_exchange_key(&mut producer_node.context, "topic_name")
                    .await;

                assert!(result.is_err());

                Ok(())
            };

            timeout(Duration::from_secs(10), test_body).await.unwrap()
        })
    }

    async fn create_secure_channel_controller(
        test_clock: TestClock,
        node: &mut TestNode,
        destination: MultiAddr,
        authority: Identifier,
    ) -> KafkaKeyExchangeControllerImpl {
        let consumer_policy_access_control =
            node.node_manager.policies().make_policy_access_control(
                node.secure_channels.identities().identities_attributes(),
                Resource::new("arbitrary-resource-name", ResourceType::KafkaConsumer),
                Action::HandleMessage,
                Env::new(),
                Some(authority.clone()),
            );

        KafkaKeyExchangeControllerImpl::new_extended(
            test_clock,
            (*node.node_manager).clone(),
            node.node_manager
                .secure_channels()
                .vault()
                .encryption_at_rest_vault
                .clone(),
            ConsumerResolution::SingleNode(destination),
            ConsumerPublishing::None,
            consumer_policy_access_control,
        )
    }
}
