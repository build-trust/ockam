use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{error::Error, NodeReply, NodeReplyResult, Reason};

use ockam_core::{Address, AddressSet, Result};

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
    trace!("Resolving worker address '{}'", addr);

    let primary_address;
    if let Some(p) = router.map.addr_map.get(addr) {
        primary_address = p.clone();
    } else {
        reply
            .send(NodeReply::no_such_worker(addr.clone()))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        return Ok(());
    }

    match router.map.internal.get(&primary_address) {
        Some(record) if record.check() => {
            reply.send(NodeReply::sender(addr.clone(), record.sender(), wrap))
        }
        Some(_) => reply.send(NodeReply::rejected(Reason::WorkerShutdown)),
        None => reply.send(NodeReply::no_such_worker(addr.clone())),
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

/// Check if an address is already in-use by another worker
pub(super) async fn check_addr_collisions(
    router: &Router,
    addrs: &AddressSet,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    if let Some(addr) = addrs.iter().fold(None, |acc, addr| {
        match (acc, router.map.internal.contains_key(addr)) {
            (None, true) => Some(addr.clone()),
            (None, false) => None,
            // If a collision was already found, ignore further collisions
            (Some(addr), _) => Some(addr),
        }
    }) {
        reply.send(NodeReply::worker_exists(addr))
    } else {
        reply.send(NodeReply::ok())
    }
    .await
    .map_err(|_| Error::InternalIOFailure.into())
}
