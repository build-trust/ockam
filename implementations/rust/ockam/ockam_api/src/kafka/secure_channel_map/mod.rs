use minicbor::{CborLen, Decode, Encode};

use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

pub(crate) mod controller;
pub(crate) mod relays;
mod secure_channels;

pub(crate) struct KafkaEncryptedContent {
    /// The encrypted content
    pub(crate) content: Vec<u8>,
    /// The secure channel identifier used to encrypt the content
    pub(crate) consumer_decryptor_address: Address,
}

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
