use crate::kafka::protocol_aware::KafkaEncryptedContent;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::{async_trait, Address};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

pub(crate) mod controller;
mod secure_channels;

/// Describe how to reach the consumer node: either directly or through a relay
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub enum ConsumerResolution {
    #[n(1)] None,
    #[n(2)] SingleNode(#[n(1)] MultiAddr),
    #[n(3)] ViaRelay(#[n(1)] MultiAddr),
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub enum ConsumerPublishing {
    #[n(1)] None,
    #[n(2)] Relay(#[n(1)] MultiAddr),
}

type TopicPartition = (String, i32);

/// Offer simple APIs to encrypt and decrypt kafka messages.
/// Underneath it creates secure channels for each topic/partition
/// and uses them to encrypt the content.
/// Multiple secure channels may be created for the same topic/partition
/// but each will be explicitly labeled.
#[async_trait]
pub(crate) trait KafkaKeyExchangeController: Send + Sync + 'static {
    /// Encrypts the content specifically for the consumer waiting for that topic name and
    /// partition.
    /// To do so, it'll create a secure channel which will be used for key exchange only.
    /// The secure channel will be created only once and then re-used, hence the first time will
    /// be slower, and may take up to few seconds.
    async fn encrypt_content(
        &self,
        context: &mut Context,
        topic_name: &str,
        partition_index: i32,
        content: Vec<u8>,
    ) -> ockam_core::Result<KafkaEncryptedContent>;

    /// Decrypts the content based on the consumer decryptor address
    /// the secure channel is expected to be already initialized.
    async fn decrypt_content(
        &self,
        context: &mut Context,
        consumer_decryptor_address: &Address,
        encrypted_content: Vec<u8>,
    ) -> ockam_core::Result<Vec<u8>>;

    /// Starts relays in the orchestrator for each {topic_name}_{partition} combination
    /// should be used only by the consumer.
    /// does nothing if they were already created, but fails it they already exist.
    async fn publish_consumer(
        &self,
        context: &mut Context,
        topic_name: &str,
        partitions: Vec<i32>,
    ) -> ockam_core::Result<()>;
}
