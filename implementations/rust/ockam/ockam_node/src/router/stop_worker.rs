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
    detached: bool,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    // Resolve any secondary address to the primary address
    let primary_address = match router.map.get_primary_address(addr) {
        Some(p) => p.clone(),
        None => {
            reply
                .send(RouterReply::no_such_address(addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    // Get the internal address record
    let record = match router.map.get_address_record_mut(&primary_address) {
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

    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    // If we are dropping a real worker, then we simply close the
    // mailbox channel to trigger a graceful worker self-shutdown.
    //
    // For detached workers (i.e. Context's without a mailbox relay
    // running) we simply drop the record
    if !detached {
        record.drop_sender();
    } else {
        router.map.free_address(primary_address);
    }

    Ok(())
}
