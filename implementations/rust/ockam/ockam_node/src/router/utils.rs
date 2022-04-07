use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::{NodeError, NodeReason, WorkerReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result, TransportType};

/// Receive an address and resolve it to a sender
///
/// This function only applies to local address types, and will
/// fail to resolve a correct address if it given a remote
/// address.
pub(super) async fn resolve(
    router: &mut Router,
    addr: &Address,
    reply: &Sender<NodeReplyResult>,
    wrap: bool,
) -> Result<()> {
    let base = format!("Resolving worker address '{:?}'...", addr);

    let primary_address = if let Some(p) = router.map.addr_map.get(addr) {
        p.clone()
    } else {
        trace!("{} FAILED; no such worker", base);
        reply
            .send(RouterReply::no_such_address(addr.clone()))
            .await
            .map_err(NodeError::from_send_err)?;

        return Ok(());
    };

    match router.map.internal.get(&primary_address) {
        Some(record) if record.check() => {
            trace!("{} OK", base);
            reply.send(RouterReply::sender(addr.clone(), record.sender(), wrap))
        }
        Some(_) => {
            trace!("{} REJECTED; worker shutting down", base);
            reply.send(RouterReply::worker_rejected(WorkerReason::Shutdown))
        }
        None => {
            trace!("{} FAILED; no such worker", base);
            reply.send(RouterReply::no_such_address(addr.clone()))
        }
    }
    .await
    .map_err(NodeError::from_send_err)?;
    Ok(())
}

pub(super) fn router_addr(router: &mut Router, tt: TransportType) -> Result<Address> {
    router
        .external
        .get(&tt)
        .cloned()
        .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())
}
