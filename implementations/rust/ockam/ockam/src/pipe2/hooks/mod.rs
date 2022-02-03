use crate::{Context, OckamMessage, Result, Routed, SystemHandler};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap},
    Address,
};

pub struct ReceiverOrdering {
    /// Set of message IDs that were received out-of-order
    journal: BTreeMap<u64, OckamMessage>,
    /// The current message index
    current: u64,
    /// Forwarding address after this stage
    next: Address,
}

impl ReceiverOrdering {
    /// Create a new hook with an internal address to forward messages
    /// to after processing them
    pub fn new(next: Address) -> Self {
        Self {
            journal: BTreeMap::new(),
            current: 0,
            next,
        }
    }

    fn compare_index(&self, index: u64) -> IndexState {
        let next = self.current + 1;

        match index {
            high if high > next => IndexState::High,
            low if low < next => IndexState::Low,
            _next => IndexState::Next,
        }
    }

    fn enqueue(&mut self, index: u64, msg: OckamMessage) -> Result<()> {
        info!("Enqueueing message with index {}", index);
        self.journal.insert(index, msg);
        Ok(())
    }
}

enum IndexState {
    Low,
    High,
    Next,
}

#[async_trait]
impl SystemHandler<Context, OckamMessage> for ReceiverOrdering {
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        Ok(())
    }
}
