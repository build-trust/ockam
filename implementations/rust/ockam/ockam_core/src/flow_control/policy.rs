use minicbor::{Decode, Encode};

/// Policy according to which a Consumer wants to receive messages from a Producer or Spawner
#[derive(Copy, Clone, Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum SpawnerFlowControlPolicy {
    /// Spawner is allowed to send only one message to the Consumer
    #[n(0)] AllowOnlyOneMessage,
    /// Spawner is allowed to send any number of messages to the Consumer
    #[n(1)] AllowMultipleMessages,
}
