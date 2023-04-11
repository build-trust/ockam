/// Policy according to which a Consumer wants to receive messages from a Producer or Spawner
#[derive(Copy, Clone, Debug)]
pub enum FlowControlPolicy {
    /// Producer is allowed to send any number of messages to the Consumer
    ProducerAllowMultiple,
    /// Spawner is allowed to send only one message to the Consumer
    SpawnerAllowOnlyOneMessage,
    /// Spawner is allowed to send any number of messages to the Consumer
    SpawnerAllowMultipleMessages,
}
