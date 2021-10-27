use super::{AddressRecord, NodeState, Router};
use crate::tokio::sync::mpsc::Sender;
use crate::{error::Error, relay::RelayMessage, NodeReply, NodeReplyResult};

use ockam_core::{AddressSet, Result};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: AddressSet,
    sender: Sender<RelayMessage>,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, sender, reply).await,
        NodeState::Stopping => reject(addrs, sender, reply).await,
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addrs: AddressSet,
    sender: Sender<RelayMessage>,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Starting new worker '{}'", addrs.first());

    // Create an address record and insert it into the internal map
    let primary_addr = addrs.first();
    let address_record = AddressRecord::new(addrs.clone(), sender);
    router.internal.insert(primary_addr.clone(), address_record);

    #[cfg(feature = "std")]
    if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
        trace!("{:#?}", router.internal);
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.internal);

    addrs.iter().for_each(|addr| {
        router.addr_map.insert(addr.clone(), primary_addr.clone());
    });

    // For now we just send an OK back -- in the future we need to
    // communicate the current executor state
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;
    Ok(())
}
