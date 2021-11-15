use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{error::Error, NodeReply, NodeReplyResult, Reason};

use ockam_core::{Address, Result};

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
    let base = format!("Resolving worker address '{}'...", addr);

    let primary_address;
    if let Some(p) = router.map.addr_map.get(addr) {
        primary_address = p.clone();
    } else {
        trace!("{} FAILED; no such worker", base);
        reply
            .send(NodeReply::no_such_worker(addr.clone()))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        return Ok(());
    }

    match router.map.internal.get(&primary_address) {
        Some(record) if record.check() => {
            trace!("{} OK", base);
            reply.send(NodeReply::sender(addr.clone(), record.sender(), wrap))
        }
        Some(_) => {
            trace!("{} REJECTED; worker shutting down", base);
            reply.send(NodeReply::rejected(Reason::WorkerShutdown))
        }
        None => {
            trace!("{} FAILED; no such worker", base);
            reply.send(NodeReply::no_such_worker(addr.clone()))
        }
    }
    .await
    .map_err(|_| Error::InternalIOFailure.into())
}

pub(super) fn router_addr(router: &mut Router, tt: u8) -> Result<Address> {
    router
        .external
        .get(&tt)
        .cloned()
        .ok_or_else(|| Error::InternalIOFailure.into())
}
