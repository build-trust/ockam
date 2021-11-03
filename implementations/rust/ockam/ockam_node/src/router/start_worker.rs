use super::{AddressMeta, AddressRecord, NodeState, Router, SenderPair};
use crate::tokio::sync::mpsc::Sender;
use crate::{error::Error, NodeReply, NodeReplyResult, Reason};

use ockam_core::{AddressSet, Result};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: AddressSet,
    senders: SenderPair,
    bare: bool,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, senders, bare, reply).await,
        NodeState::Stopping(_) => reject(reply).await,
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addrs: AddressSet,
    senders: SenderPair,
    bare: bool,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    info!("Starting new worker '{}'", addrs.first());
    let SenderPair { msgs, ctrl } = senders;

    // Create an address record and insert it into the internal map
    let primary_addr = addrs.first();
    let address_record = AddressRecord::new(
        addrs.clone(),
        msgs,
        ctrl,
        AddressMeta {
            processor: false,
            bare,
        },
    );
    router
        .map
        .internal
        .insert(primary_addr.clone(), address_record);

    #[cfg(feature = "std")]
    if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
        trace!("{:#?}", router.map.internal);
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.map.internal);

    addrs.iter().for_each(|addr| {
        router
            .map
            .addr_map
            .insert(addr.clone(), primary_addr.clone());
    });

    // For now we just send an OK back -- in the future we need to
    // communicate the current executor state
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;
    Ok(())
}

async fn reject(reply: &Sender<NodeReplyResult>) -> Result<()> {
    trace!("StartWorker command rejected: node shutting down");
    reply
        .send(NodeReply::rejected(Reason::NodeShutdown))
        .await
        .map_err(|_| Error::InternalIOFailure)?;
    Ok(())
}
