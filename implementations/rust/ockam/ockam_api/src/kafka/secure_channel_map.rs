use crate::kafka::{KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS, ORCHESTRATOR_KAFKA_CONSUMERS};
use crate::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use crate::nodes::models::secure_channel::{
    CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
};
use crate::nodes::NODEMANAGER_ADDR;
use crate::DefaultAddress;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Message;
use ockam_core::{async_trait, route, Address, AllowAll, Error, Result, Routed, Worker};
use ockam_identity::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
    SecureChannelRegistryEntry, SecureChannels,
};
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::tokio::sync::Mutex;
use ockam_node::Context;
use serde::{Deserialize, Serialize};

pub(crate) struct KafkaEncryptedContent {
    /// The encrypted content
    pub(crate) content: Vec<u8>,
    /// The secure channel id used to encrypt the content
    pub(crate) secure_channel_id: UniqueSecureChannelId,
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

    /// Decrypts the content based on the unique secure channel identifier
    /// the secure channel is expected to be already initialized.
    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        secure_channel_id: UniqueSecureChannelId,
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
        let buffer: Vec<u8> = context
            .send_and_receive(
                route![NODEMANAGER_ADDR],
                Request::post("/node/forwarder")
                    .body(CreateForwarder::at_project(forwarder_service, Some(alias)))
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

///Unique identifier for a specific secure_channel.
/// Used in order to distinguish between secure channels created between
/// the same identities.
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
struct SecureChannelIdentifierMessage {
    secure_channel_identifier: UniqueSecureChannelId,
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

/// An identifier of the secure channel **instance**
pub(crate) type UniqueSecureChannelId = u64;
type TopicPartition = (String, i32);
struct InnerSecureChannelControllerImpl<F: ForwarderCreator> {
    //we are using encryptor api address as unique _local_ identifier
    //of the secure channel
    id_encryptor_map: HashMap<UniqueSecureChannelId, Address>,
    topic_encryptor_map: HashMap<TopicPartition, (UniqueSecureChannelId, Address)>,
    project_multiaddr: MultiAddr,
    topic_forwarder_set: HashSet<TopicPartition>,
    forwarder_creator: F,
    secure_channels: Arc<SecureChannels>,
}

impl KafkaSecureChannelControllerImpl<NodeManagerForwarderCreator> {
    pub(crate) fn new(
        secure_channels: Arc<SecureChannels>,
        project_multiaddr: MultiAddr,
    ) -> KafkaSecureChannelControllerImpl<NodeManagerForwarderCreator> {
        let mut orchestrator_multiaddr = project_multiaddr.clone();
        orchestrator_multiaddr
            .push_back(Service::new(ORCHESTRATOR_KAFKA_CONSUMERS))
            .unwrap();
        Self::new_extended(
            secure_channels,
            project_multiaddr,
            NodeManagerForwarderCreator {
                orchestrator_multiaddr,
            },
        )
    }
}

impl<F: ForwarderCreator> KafkaSecureChannelControllerImpl<F> {
    /// to manually specify `ForwarderCreator`, for testing purposes
    pub(crate) fn new_extended(
        secure_channels: Arc<SecureChannels>,
        project_multiaddr: MultiAddr,
        forwarder_creator: F,
    ) -> KafkaSecureChannelControllerImpl<F> {
        Self {
            inner: Arc::new(Mutex::new(InnerSecureChannelControllerImpl {
                id_encryptor_map: Default::default(),
                topic_encryptor_map: Default::default(),
                topic_forwarder_set: Default::default(),
                secure_channels,
                forwarder_creator,
                project_multiaddr,
            })),
        }
    }

    pub(crate) async fn create_consumer_listener(&self, context: &Context) -> Result<()> {
        context
            .start_worker(
                Address::from_string(KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS),
                SecureChannelControllerListener::<F> {
                    controller: self.clone(),
                },
                AllowAll,
                AllowAll,
            )
            .await
    }

    pub(crate) fn into_trait(self) -> Arc<dyn KafkaSecureChannelController> {
        Arc::new(self)
    }

    //add a mapping from remote producer
    async fn add_mapping(&self, id: UniqueSecureChannelId, encryptor_address: Address) {
        self.inner
            .lock()
            .await
            .id_encryptor_map
            .insert(id, encryptor_address);
    }
}

struct SecureChannelControllerListener<F: ForwarderCreator> {
    controller: KafkaSecureChannelControllerImpl<F>,
}

#[ockam::worker]
impl<F: ForwarderCreator> Worker for SecureChannelControllerListener<F> {
    type Message = SecureChannelIdentifierMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        //todo: is there a better way to extract it from the context?
        let encryptor_address = message.return_route().next().cloned()?;

        self.controller
            .add_mapping(message.secure_channel_identifier, encryptor_address.clone())
            .await;

        context.send(message.return_route(), ()).await
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
                        CredentialExchangeMode::None,
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

    ///returns encryptor api address
    async fn get_or_create_secure_channel_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition: i32,
    ) -> Result<(UniqueSecureChannelId, SecureChannelRegistryEntry)> {
        //here we should have the orchestrator address and expect forwarders to be
        // present in the orchestrator with the format "consumer_{topic_name}_{partition}"

        let topic_partition_key = (topic_name.to_string(), partition);
        //consumer__ prefix is added by the orchestrator
        let topic_partition_address = format!("consumer__{topic_name}_{partition}");

        let mut inner = self.inner.lock().await;

        let (random_unique_id, encryptor_address) = {
            if let Some(encryptor_address) = inner.topic_encryptor_map.get(&topic_partition_key) {
                encryptor_address.clone()
            } else {
                trace!("creating new secure channel to {topic_partition_address}");

                let mut destination = inner.project_multiaddr.clone();
                destination.push_back(Service::new(topic_partition_address.clone()))?;
                destination.push_back(Service::new(DefaultAddress::SECURE_CHANNEL_LISTENER))?;

                let encryptor_address =
                    Self::request_secure_channel_creation(context, destination).await?;

                trace!("created secure channel to {topic_partition_address}");

                let random_unique_id: UniqueSecureChannelId = rand::random();
                inner.topic_encryptor_map.insert(
                    topic_partition_key,
                    (random_unique_id, encryptor_address.clone()),
                );

                let message = SecureChannelIdentifierMessage {
                    secure_channel_identifier: random_unique_id,
                };

                //communicate to the other end the random id associated with this
                //secure channel, and wait to an empty reply to avoid race conditions
                //on the order of encryption/decryption of messages
                context
                    .send_and_receive(
                        route![
                            encryptor_address.clone(),
                            KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS
                        ],
                        message,
                    )
                    .await?;

                trace!("assigned id {random_unique_id} to {topic_partition_address}");
                (random_unique_id, encryptor_address)
            }
        };

        inner
            .secure_channels
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&encryptor_address)
            .map(|entry| (random_unique_id, entry))
            .ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    format!("cannot find secure channel address `{encryptor_address}` in local registry"),
                )
            })
    }

    ///return decryptor api address
    async fn get_secure_channel_for(
        &self,
        secure_channel_id: UniqueSecureChannelId,
    ) -> Result<SecureChannelRegistryEntry> {
        let inner = self.inner.lock().await;
        if let Some(encryptor_address) = inner.id_encryptor_map.get(&secure_channel_id) {
            inner
                .secure_channels
                .secure_channel_registry()
                .get_channel_list()
                .iter()
                .find(|entry| {
                    entry.encryptor_messaging_address() == encryptor_address
                        && !entry.is_initiator()
                })
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        Origin::Channel,
                        Kind::Unknown,
                        "secure channel no longer exists",
                    )
                })
        } else {
            Err(Error::new(
                Origin::Channel,
                Kind::Unknown,
                "missing secure channel",
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
        let (unique_id, secure_channel_entry) = self
            .get_or_create_secure_channel_for(context, topic_name, partition_id)
            .await?;

        trace!("encrypting content with {unique_id}");
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

        trace!("encrypted content with {unique_id}");
        Ok(KafkaEncryptedContent {
            content: encrypted_content,
            secure_channel_id: unique_id,
        })
    }

    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        secure_channel_id: UniqueSecureChannelId,
        encrypted_content: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let secure_channel_entry = self.get_secure_channel_for(secure_channel_id).await?;

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
