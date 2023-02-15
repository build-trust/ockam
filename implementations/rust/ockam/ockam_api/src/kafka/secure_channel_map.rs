use bytes::{Bytes, BytesMut};
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Message;
use ockam_core::{async_trait, route, Address, AllowAll, CowBytes, Error, Routed, TypeTag, Worker};
use ockam_identity::api::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, SecureChannelRegistryEntry, TrustEveryonePolicy,
};
use ockam_node::compat::tokio::sync::Mutex;
use ockam_node::Context;
use serde::{Deserialize, Serialize};

///This is proxy trait to avoid propagating the vault implementation.
/// Offer simple APIs to encrypt and decrypt kafka messages.
/// Underneath it create secure channels for each topic/partition
/// and uses them to encrypt the content.
/// Multiple secure channels may be created for the same topic/partition
/// but each will be explicitly label.
#[async_trait]
pub(crate) trait KafkaSecureChannelController: Send + Sync {
    async fn encrypt_content_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_id: i32,
        content: Vec<u8>,
    ) -> Result<(UniqueSecureChannelId, Vec<u8>), Error>;

    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        secure_channel_id: UniqueSecureChannelId,
        encrypted_content: Vec<u8>,
    ) -> Result<Vec<u8>, Error>;
}

#[derive(Debug)]
pub(crate) struct KafkaSecureChannelControllerImpl<V: IdentityVault, S: AuthenticatedStorage> {
    inner: Arc<Mutex<InnerSecureChannelControllerImpl<V, S>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rustfmt::skip]
///Unique identifier for a specific secure_channel.
/// Used in order to distinguish between secure channels created between
/// the same identities.
struct SecureChannelIdentifier {
    secure_channel_identifier: UniqueSecureChannelId,
}

pub(crate) type UniqueSecureChannelId = u64;
type TopicPartition = (String, i32);
struct InnerSecureChannelControllerImpl<V: IdentityVault, S: AuthenticatedStorage> {
    //we are using encryptor api address as unique _local_ identifier
    //of the secure channel
    id_encryptor_map: HashMap<UniqueSecureChannelId, Address>,
    topic_encryptor_map: HashMap<TopicPartition, (UniqueSecureChannelId, Address)>,
    identity: Identity<V, S>,
}

impl<V: IdentityVault, S: AuthenticatedStorage> KafkaSecureChannelControllerImpl<V, S> {
    pub(crate) fn new(identity: Identity<V, S>) -> KafkaSecureChannelControllerImpl<V, S> {
        Self {
            inner: Arc::new(Mutex::new(InnerSecureChannelControllerImpl {
                id_encryptor_map: Default::default(),
                topic_encryptor_map: Default::default(),
                identity,
            })),
        }
    }

    pub(crate) async fn start_consumer_listener(&self, context: &mut Context) -> Result<(), Error> {
        context
            .start_worker(
                Address::random_tagged("kafka_secure_channel_controller"),
                SecureChannelControllerListener {
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
    pub(crate) async fn add_mapping(&self, id: UniqueSecureChannelId, encryptor_address: Address) {
        self.inner
            .lock()
            .await
            .id_encryptor_map
            .insert(id, encryptor_address);
    }
}

struct SecureChannelControllerListener<V: IdentityVault, S: AuthenticatedStorage> {
    controller: KafkaSecureChannelControllerImpl<V, S>,
}

#[ockam::worker]
impl<V: IdentityVault, S: AuthenticatedStorage> Worker for SecureChannelControllerListener<V, S> {
    type Message = SecureChannelIdentifier;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        //todo: is there a better way to extract it from the context?
        let encryptor_address = message.return_route().next()?;

        self.controller
            .add_mapping(message.secure_channel_identifier, encryptor_address.clone())
            .await;

        context.send(message.return_route(), ()).await
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> KafkaSecureChannelControllerImpl<V, S> {
    ///returns encryptor api address
    async fn get_or_create_secure_channel_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_id: i32,
    ) -> Result<(UniqueSecureChannelId, SecureChannelRegistryEntry), Error> {
        //here we should have the orchestrator address
        // and expect forwarders to be present in the orchestrator
        // with a format similar to "kafka_consumer_forwarder_{partition}_{topic_name}"

        // for this iteration we will expect to find "kafka_consumer_secure_channel" _locally_
        let topic_partition_key = (topic_name.to_string(), partition_id);
        let mut inner = self.inner.lock().await;

        let (random_unique_id, encryptor_address) = {
            if let Some(encryptor_address) = inner.topic_encryptor_map.get(&topic_partition_key) {
                encryptor_address.clone()
            } else {
                let encryptor_address = inner
                    .identity
                    .create_secure_channel(
                        route!["kafka_secure_channel_controller"],
                        TrustEveryonePolicy,
                    )
                    .await?;

                let random_unique_id: UniqueSecureChannelId = rand::random();
                inner.topic_encryptor_map.push(
                    topic_partition_key,
                    (random_unique_id, encryptor_address.clone()),
                );

                let message = SecureChannelIdentifier {
                    #[cfg(feature = "tag")]
                    tag: TypeTag,
                    secure_channel_identifier: random_unique_id,
                };

                //communicate to the other end the random id associated with this
                //secure channel, and wait to an empty reply to avoid race conditions
                //on the order of encryption/decryption of messages
                context
                    .send_and_receive(
                        route![encryptor_address.clone(), "kafka_secure_channel_controller"],
                        message,
                    )
                    .await?;

                (random_unique_id, encryptor_address)
            }
        };

        inner
            .identity
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&encryptor_address)
            .map(|entry| (random_unique_id, entry))
            .ok_or_else(|| Error::new(Origin::Channel, Kind::Unknown, "secure channel down"))
    }

    ///return decryptor api address
    async fn get_secure_channel_for(
        &self,
        secure_channel_id: UniqueSecureChannelId,
    ) -> Result<SecureChannelRegistryEntry, Error> {
        let inner = self.inner.lock().await;
        if let Some(encryptor_address) = inner.id_encryptor_map.get(&secure_channel_id) {
            inner
                .identity
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
            Error::new(Origin::Channel, Kind::Unknown, "missing secure channel")
        }
    }
}

#[async_trait]
impl<V: IdentityVault, S: AuthenticatedStorage> KafkaSecureChannelController
    for KafkaSecureChannelControllerImpl<V, S>
{
    async fn encrypt_content_for(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_id: i32,
        content: Vec<u8>,
    ) -> Result<(UniqueSecureChannelId, Vec<u8>), Error> {
        let (unique_id, secure_channel_entry) = self
            .get_or_create_secure_channel_for(context, topic_name, partition_id)
            .await?;

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

        Ok((unique_id, encrypted_content))
    }

    async fn decrypt_content_for(
        &self,
        context: &mut Context,
        secure_channel_id: UniqueSecureChannelId,
        encrypted_content: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let secure_channel_entry = self.get_secure_channel_for(secure_channel_id)?;

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
}
