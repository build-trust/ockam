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
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    let primary_address = if let Some(p) = router.map.addr_map.get(addr) {
        p.clone()
    } else {
        reply
            .send(RouterReply::no_such_address(addr.clone()))
            .await
            .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

        return Ok(());
    };

    let record = match router.map.internal.remove(&primary_address) {
        Some(rec) => rec,
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

    Ok(())
}
