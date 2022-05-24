use super::Router;
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    addr: &Address,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    let primary_address = match router.map.addr_map.get(addr) {
        Some(p) => p.clone(),
        None => {
            reply
                .send(RouterReply::no_such_address(addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    let record = match router.map.internal.get_mut(&primary_address) {
        Some(r) => r,
        None => {
            // Actually should not happen
            reply
                .send(RouterReply::no_such_address(addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    for addr in record.address_set().iter() {
        router.map.addr_map.remove(addr);
    }

    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    // Drop worker's Sender to close the worker's mailbox channel
    // and trigger the worker to start a graceful self-shutdown.
    record.sender_drop();

    Ok(())
}
