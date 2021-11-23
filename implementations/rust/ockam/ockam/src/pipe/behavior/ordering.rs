use crate::{
    pipe::{BehaviorHook, PipeModifier},
    protocols::pipe::{internal::InternalCmd, PipeMessage},
    Context,
};
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{
    async_trait, compat::collections::BTreeMap, Address, LocalMessage, Result, Route,
};

#[derive(Default, Clone)]
pub struct ReceiverOrdering {
    journal: BTreeMap<u64, PipeMessage>,
    current: u64,
}

/// Encode the relationship between two indices
enum IndexState {
    Low,
    High,
    Next,
}

impl ReceiverOrdering {
    pub fn new() -> Self {
        Self {
            journal: BTreeMap::new(),
            current: 0,
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

    fn enqueue(&mut self, index: u64, msg: &PipeMessage) -> Result<PipeModifier> {
        info!("Enqueueing message with index {}", index);
        self.journal.insert(index, msg.clone());
        Ok(PipeModifier::Drop)
    }

    async fn forward(
        &mut self,
        ctx: &mut Context,
        index: u64,
        msg: &PipeMessage,
    ) -> Result<PipeModifier> {
        debug!("Forwarding message with index {}", index);

        // First send the currently handled message
        let curr = crate::pipe::unpack_pipe_message(msg)?;
        debug!("Forwarding message to {:?}", curr.transport().onward_route);
        ctx.forward(curr).await?;

        // Then process the journal to get all queued messages that
        // are still strictly ordered (meaning there is no gap in
        // their indices)
        let mut send_stack = vec![];
        process_journal(&mut send_stack, &mut self.current, &mut self.journal)?;

        // Then send every message in the send stack
        for msg in send_stack {
            ctx.forward(msg).await?;
        }

        // Indicate to the pipe receiver should drop this message
        // because we have already sent it
        Ok(PipeModifier::Drop)
    }
}

/// Walk through the journal until we reach a gap in the indices
///
/// We pass in a send buffer because async recursion is hard.
fn process_journal(
    send_stack: &mut Vec<LocalMessage>,
    curr: &mut u64,
    j: &mut BTreeMap<u64, PipeMessage>,
) -> Result<()> {
    *curr += 1;
    if let Some(ref msg) = j.remove(curr) {
        send_stack.push(crate::pipe::unpack_pipe_message(msg)?);
        process_journal(send_stack, curr, j)?;
    }
    Ok(())
}

#[async_trait]
impl BehaviorHook for ReceiverOrdering {
    async fn on_external(
        &mut self,
        _: Address,
        _: Route,
        ctx: &mut Context,
        msg: &PipeMessage,
    ) -> Result<PipeModifier> {
        let index = msg.index.u64();
        match self.compare_index(index) {
            IndexState::Low => {
                warn!("Ignoring message with index {}", index);
                Ok(PipeModifier::Drop)
            }
            IndexState::High => self.enqueue(index, msg),
            IndexState::Next => self.forward(ctx, index, msg).await,
        }
    }

    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &InternalCmd,
    ) -> Result<()> {
        Ok(())
    }
}
