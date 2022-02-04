use crate::{Context, OckamError, OckamMessage, Result, Routed, SystemHandler};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, string::String},
    Address,
};

#[derive(Default)]
pub struct ReceiverOrdering {
    /// Set of message IDs that were received out-of-order
    journal: BTreeMap<u64, OckamMessage>,
    /// The current message index
    current: u64,
    /// Forwarding address after this stage
    next: Option<Address>,
}

impl ReceiverOrdering {
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
    async fn initialize(
        &mut self,
        ctx: &mut Context,
        routes: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        self.next = Some(
            routes
                .remove("default")
                .ok_or(OckamError::SystemInvalidConfiguration)?,
        );
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        Ok(())
    }
}
