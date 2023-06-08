use crate::kafka::KAFKA_OUTLET_CONSUMERS;
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
    DeleteSecureChannelRequest, DeleteSecureChannelResponse,
};
use crate::nodes::NODEMANAGER_ADDR;
use crate::DefaultAddress;
use minicbor::Decoder;
use ockam_abac::AbacAccessControl;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, Address, Error, Result};
use ockam_identity::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
    SecureChannelRegistryEntry, SecureChannels, TRUST_CONTEXT_ID,
};
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::tokio::sync::Mutex;
use ockam_node::compat::tokio::sync::MutexGuard;
use ockam_node::Context;

pub(crate) struct KafkaEncryptedContent {
    /// The encrypted content
    pub(crate) content: Vec<u8>,
    /// The secure channel identifier used to encrypt the content
    pub(crate) consumer_decryptor_address: Address,
}

/// Offer simple APIs to encrypt and decrypt kafka messages.
/// Underneath it creates secure channels for each topic/partition
/// and uses them to encrypt the content.
/// Multiple secure channels may be created for the same topic/partition
/// but each will be explicitly labelled.
/// It's the same for both producer and consumer although it could be split
/// into two distinct implementations.
/// This is a proxy trait to avoid propagating the vault implementation.
#[async_trait]
pub(crate) trait KafkaSecureChannelController: Send + Sync {
    /// Encrypts the content specifically for the consumer waiting for that topic name and
    /// partition.
    /// To do so it'll create a secure channel which will be used for key exchange only.
    /// The secure channel will be created only once and then re-used, hence the first time will
    /// be slower, and may take up to few seconds.
    async fn encrypt_content_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_id: i32,
        content: Vec<u8>,
    ) -> Result<KafkaEncryptedContent>;

    /// Decrypts the content based on the consumer decryptor address
    /// the secure channel is expected to be already initialized.
    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        consumer_decryptor_address: &Address,
        encrypted_content: Vec<u8>,
    ) -> Result<Vec<u8>>;

    /// Starts forwarders in the orchestrator for each {topic_name}_{partition} combination
    /// should be used only by the consumer.
    /// does nothing if they were already created, but fails it they already exist.
    async fn start_forwarders_for(
        &self,
        context: &mut Context,
        topic_id: &str,
        partitions: Vec<i32>,
    ) -> Result<()>;
}

#[async_trait]
pub(crate) trait ForwarderCreator: Send + Sync + 'static {
    async fn create_forwarder(&self, context: &Context, alias: String) -> Result<()>;
}

pub(crate) struct NodeManagerForwarderCreator {
    orchestrator_multiaddr: MultiAddr,
}

impl NodeManagerForwarderCreator {
    async fn request_forwarder_creation(
        context: &Context,
        forwarder_service: MultiAddr,
        alias: String,
    ) -> Result<()> {
        let is_rust = true;
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/forwarder")
                    .body(CreateForwarder::at_node(
                        forwarder_service,
                        Some(alias),
                        is_rust,
                        None,
                    ))
                    .to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: Response = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot create forwarder: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid create forwarder response",
            ))
        } else {
            let remote_forwarder_information: ForwarderInfo = decoder.decode()?;
            trace!("remote forwarder created: {remote_forwarder_information:?}");
            Ok(())
        }
    }
}

#[async_trait]
impl ForwarderCreator for NodeManagerForwarderCreator {
    async fn create_forwarder(&self, context: &Context, alias: String) -> Result<()> {
        trace!("creating remote forwarder for: {alias}");
        Self::request_forwarder_creation(context, self.orchestrator_multiaddr.clone(), alias)
            .await?;
        Ok(())
    }
}

pub(crate) struct KafkaSecureChannelControllerImpl<F: ForwarderCreator> {
    inner: Arc<Mutex<InnerSecureChannelControllerImpl<F>>>,
}

//had to manually implement since #[derive(Clone)] doesn't work well in this situation
impl<F: ForwarderCreator> Clone for KafkaSecureChannelControllerImpl<F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

type TopicPartition = (String, i32);
struct InnerSecureChannelControllerImpl<F: ForwarderCreator> {
    // we identity the secure channel instance by using the decryptor of the consumer
    // which is known to both parties
    topic_encryptor_map: HashMap<TopicPartition, Address>,
    outlet_node_multiaddr: MultiAddr,
    topic_forwarder_set: HashSet<TopicPartition>,
    forwarder_creator: F,
    secure_channels: Arc<SecureChannels>,
    access_control: AbacAccessControl,
}

impl KafkaSecureChannelControllerImpl<NodeManagerForwarderCreator> {
    pub(crate) fn new(
        secure_channels: Arc<SecureChannels>,
        outlet_node_multiaddr: MultiAddr,
        trust_context_id: String,
    ) -> KafkaSecureChannelControllerImpl<NodeManagerForwarderCreator> {
        let mut orchestrator_multiaddr = outlet_node_multiaddr.clone();
        orchestrator_multiaddr
            .push_back(Service::new(KAFKA_OUTLET_CONSUMERS))
            .unwrap();
        Self::new_extended(
            secure_channels,
            outlet_node_multiaddr,
            NodeManagerForwarderCreator {
                orchestrator_multiaddr,
            },
            trust_context_id,
        )
    }
}

impl<F: ForwarderCreator> KafkaSecureChannelControllerImpl<F> {
    /// to manually specify `ForwarderCreator`, for testing purposes
    pub(crate) fn new_extended(
        secure_channels: Arc<SecureChannels>,
        outlet_node_multiaddr: MultiAddr,
        forwarder_creator: F,
        trust_context_id: String,
    ) -> KafkaSecureChannelControllerImpl<F> {
        let access_control = AbacAccessControl::create(
            secure_channels.identities().repository(),
            TRUST_CONTEXT_ID,
            &trust_context_id,
        );

        Self {
            inner: Arc::new(Mutex::new(InnerSecureChannelControllerImpl {
                topic_encryptor_map: Default::default(),
                topic_forwarder_set: Default::default(),
                secure_channels,
                forwarder_creator,
                outlet_node_multiaddr,
                access_control,
            })),
        }
    }

    pub(crate) fn into_trait(self) -> Arc<dyn KafkaSecureChannelController> {
        Arc::new(self)
    }
}

impl<F: ForwarderCreator> KafkaSecureChannelControllerImpl<F> {
    async fn request_secure_channel_creation(
        context: &Context,
        destination: MultiAddr,
    ) -> Result<Address> {
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/secure_channel")
                    .body(CreateSecureChannelRequest::new(
                        &destination,
                        None,
                        CredentialExchangeMode::Mutual,
                        None,
                        None,
                    ))
                    .to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: Response = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot create secure channel: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid create secure channel response",
            ))
        } else {
            let secure_channel_response: CreateSecureChannelResponse = decoder.decode()?;
            Ok(secure_channel_response.addr)
        }
    }

    async fn request_secure_channel_deletion(
        context: &Context,
        encryptor_address: &Address,
    ) -> Result<()> {
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::delete("/node/secure_channel")
                    .body(DeleteSecureChannelRequest::new(encryptor_address))
                    .to_vec()?,
            )
            .await?;

        let mut decoder = Decoder::new(&buffer);
        let response: Response = decoder.decode()?;

        let status = response.status().unwrap_or(Status::InternalServerError);
        if status != Status::Ok {
            return Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                format!("cannot delete secure channel: {}", status),
            ));
        }
        if !response.has_body() {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "invalid delete secure channel response",
            ))
        } else {
            let _secure_channel_response: DeleteSecureChannelResponse = decoder.decode()?;
            Ok(())
        }
    }

    ///returns encryptor api address
    async fn get_or_create_secure_channel_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition: i32,
    ) -> Result<SecureChannelRegistryEntry> {
        // here we should have the orchestrator address and expect forwarders to be
        // present in the orchestrator with the format "consumer__{topic_name}_{partition}"

        let topic_partition_key = (topic_name.to_string(), partition);
        //consumer__ prefix is added by the orchestrator
        let topic_partition_address = format!("consumer__{topic_name}_{partition}");

        let mut inner = self.inner.lock().await;

        let encryptor_address = {
            if let Some(encryptor_address) = inner.topic_encryptor_map.get(&topic_partition_key) {
                encryptor_address.clone()
            } else {
                debug!("creating new secure channel to {topic_partition_address}");

                let mut destination = inner.outlet_node_multiaddr.clone();
                destination.push_back(Service::new(topic_partition_address.clone()))?;
                destination.push_back(Service::new(DefaultAddress::SECURE_CHANNEL_LISTENER))?;

                let producer_encryptor_address =
                    Self::request_secure_channel_creation(context, destination).await?;

                match Self::validate_consumer_credentials(&inner, &producer_encryptor_address).await
                {
                    Ok(producer_encryptor_address) => producer_encryptor_address,
                    Err(error) => {
                        Self::request_secure_channel_deletion(context, &producer_encryptor_address)
                            .await?;
                        return Err(error);
                    }
                };

                inner
                    .topic_encryptor_map
                    .insert(topic_partition_key, producer_encryptor_address.clone());

                debug!("created secure channel to {topic_partition_address}");
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
        inner: &MutexGuard<'_, InnerSecureChannelControllerImpl<F>>,
        producer_encryptor_address: &Address,
    ) -> Result<Address> {
        let record = inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_encryptor_address(producer_encryptor_address);

        if let Some(entry) = record {
            let authorized = inner
                .access_control
                .is_identity_authorized(entry.their_id())
                .await?;

            if authorized {
                Ok(producer_encryptor_address.clone())
            } else {
                Err(Error::new(
                    Origin::Transport,
                    Kind::Invalid,
                    "unauthorized secure channel for consumer",
                ))
            }
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::Unknown,
                "cannot find secure channel entry",
            ))
        }
    }

    ///return decryptor api address
    async fn get_secure_channel_for(
        &self,
        consumer_decryptor_address: &Address,
    ) -> Result<SecureChannelRegistryEntry> {
        let inner = self.inner.lock().await;
        let entry = inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_decryptor_address(consumer_decryptor_address)
            .ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    format!(
                        "secure channel decrypt doesn't exists: {}",
                        consumer_decryptor_address.address()
                    ),
                )
            })?;

        let authorized = inner
            .access_control
            .is_identity_authorized(entry.their_id())
            .await?;

        if authorized {
            Ok(entry)
        } else {
            Err(Error::new(
                Origin::Transport,
                Kind::Invalid,
                "unauthorized secure channel",
            ))
        }
    }
}

#[async_trait]
impl<F: ForwarderCreator> KafkaSecureChannelController for KafkaSecureChannelControllerImpl<F> {
    async fn encrypt_content_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_id: i32,
        content: Vec<u8>,
    ) -> Result<KafkaEncryptedContent> {
        let secure_channel_entry = self
            .get_or_create_secure_channel_for(context, topic_name, partition_id)
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

    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        consumer_decryptor_address: &Address,
        encrypted_content: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let secure_channel_entry = self
            .get_secure_channel_for(consumer_decryptor_address)
            .await?;

        let decrypt_response = context
            .send_and_receive(
                route![secure_channel_entry.decryptor_api_address().clone()],
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

    async fn start_forwarders_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partitions: Vec<i32>,
    ) -> Result<()> {
        let mut inner = self.inner.lock().await;

        for partition in partitions {
            let topic_key: TopicPartition = (topic_name.to_string(), partition);
            if inner.topic_forwarder_set.contains(&topic_key) {
                continue;
            }
            let alias = format!("{topic_name}_{partition}");
            inner
                .forwarder_creator
                .create_forwarder(context, alias)
                .await?;
            inner.topic_forwarder_set.insert(topic_key);
        }

        Ok(())
    }
}
