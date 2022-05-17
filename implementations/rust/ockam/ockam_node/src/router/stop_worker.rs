use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    addr: &Address,
    bare: bool,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    // Resolve any secondary address to the primary address
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

    // Get the internal address record
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

    // Remove all secondary addresses
    for addr in record.address_set().iter() {
        router.map.addr_map.remove(addr);
    }

    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    // If we are dropping a real worker, then we simply close the
    // mailbox channel to trigger a graceful worker self-shutdown.
    //
    // For bare workers (i.e. Context's without a mailbox relay
    // running) we simply drop the record
    if !bare {
        record.sender_drop();
    } else {
        router.map.free_address(primary_address);
    }

    Ok(())
}
