use minicbor::{Decode, Encode};

/// Policy according to which a Consumer wants to receive messages from a Producer or Spawner
#[derive(Copy, Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum FlowControlPolicy {
    /// Producer is allowed to send any number of messages to the Consumer
    #[n(0)] ProducerAllowMultiple,
    /// Spawner is allowed to send only one message to the Consumer
    #[n(1)] SpawnerAllowOnlyOneMessage,
    /// Spawner is allowed to send any number of messages to the Consumer
    #[n(2)] SpawnerAllowMultipleMessages,
}
