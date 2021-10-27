use super::{AddressRecord, NodeState, Router};
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::Error,
    relay::{RelayMessage, PROC_ADDR_SUFFIX},
    NodeReply, NodeReplyResult,
};

use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    addr: &Address,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    let primary_address;
    if let Some(p) = router.addr_map.get(addr) {
        primary_address = p.clone();
    } else {
        reply
            .send(NodeReply::no_such_worker(addr.clone()))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        return Ok(());
    }

    let record;
    if let Some(r) = router.internal.remove(&primary_address) {
        record = r;
    } else {
        // Actually should not happen
        reply
            .send(NodeReply::no_such_worker(addr.clone()))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        return Ok(());
    }

    for addr in record.address_set().iter() {
        router.addr_map.remove(&addr);
    }

    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;

    Ok(())
}
