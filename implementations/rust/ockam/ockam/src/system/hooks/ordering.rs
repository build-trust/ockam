use crate::{
    monotonic::Monotonic, Context, OckamError, OckamMessage, Result, Routed, SystemHandler,
};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, string::String, vec::Vec},
    Address, Decodable, Encodable, Message,
};
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize)]
struct Index(u64);

#[derive(Clone, Default)]
pub struct ReceiverOrdering {
    /// Set of message IDs that were received out-of-order
    journal: BTreeMap<u64, OckamMessage>,
    /// The current message index
    current: u64,
    /// Forwarding address after this stage
    next: Option<Address>,
}

/// Walk through the journal until we reach a gap in the indices
///
/// We pass in a send buffer because async recursion is hard.
fn process_journal(
    send_stack: &mut Vec<OckamMessage>,
    curr: &mut u64,
    j: &mut BTreeMap<u64, OckamMessage>,
) -> Result<()> {
    *curr += 1;
    if let Some(msg) = j.remove(curr) {
        send_stack.push(msg);
        process_journal(send_stack, curr, j)?;
    }
    Ok(())
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

    async fn forward(
        &mut self,
        ctx: &mut Context,
        mut index: u64,
        msg: OckamMessage,
    ) -> Result<()> {
        debug!("Forwarding message with index {}", index);

        // First forward the currently handled message to the next hop
        let next_addr = self.next.as_ref().unwrap().clone();
        ctx.send(next_addr.clone(), msg).await?;

        // Then process the journal to get all queued messages that
        // are still strictly ordered (meaning there is no gap in
        // their indices)
        let mut send_stack = vec![];
        process_journal(&mut send_stack, &mut self.current, &mut self.journal)?;

        for msg in send_stack {
            ctx.send(next_addr.clone(), msg).await?;
        }

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

    async fn handle_message(
        &mut self,
        self_addr: Address,
        ctx: &mut Context,
        msg: Routed<OckamMessage>,
    ) -> Result<()> {
        trace!("ReceiverOrdering: handling incoming message");
        let Index(index) = msg
            .scope
            .get(0)
            .ok_or_else(|| OckamError::InvalidParameter.into())
            .and_then(|idx| Index::decode(idx))?;

        // Peel off this layer of message
        let inner = msg.body().peel()?;

        match self.compare_index(index) {
            IndexState::Low => {
                warn!("Ignoring message with index (too low): {}", index);
                Ok(())
            }
            IndexState::High => self.enqueue(index, inner),
            IndexState::Next => self.forward(ctx, index, inner).await,
        }
    }
}

pub struct SenderOrdering {
    index: Monotonic,
    next: Option<Address>,
}

impl Default for SenderOrdering {
    fn default() -> Self {
        Self {
            index: Monotonic::from(1),
            next: None,
        }
    }
}

impl Clone for SenderOrdering {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[async_trait]
impl SystemHandler<Context, OckamMessage> for SenderOrdering {
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

    async fn handle_message(
        &mut self,
        self_addr: Address,
        ctx: &mut Context,
        msg: Routed<OckamMessage>,
    ) -> Result<()> {
        let inner_msg = msg.body();
        let index = Index(self.index.next() as u64);
        let outer_msg = OckamMessage::wrap(inner_msg)?.scope_data(index.encode()?);

        ctx.send(self.next.as_ref().unwrap().clone(), outer_msg)
            .await
    }
}
