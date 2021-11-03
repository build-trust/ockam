use super::{AddressMeta, AddressRecord, NodeState, Router, SenderPair};
use crate::tokio::sync::mpsc::Sender;
use crate::{error::Error, NodeReply, NodeReplyResult, Reason};

use ockam_core::{Address, Result};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: Address,
    senders: SenderPair,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, senders, reply).await,
        NodeState::Stopping(_) => reject(reply).await,
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addr: Address,
    senders: SenderPair,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    info!("Starting new processor '{}'", &addr);
    let SenderPair { msgs, ctrl } = senders;

    let record = AddressRecord::new(
        addr.clone().into(),
        msgs,
        ctrl,
        AddressMeta {
            processor: true,
            bare: false,
        },
    );

    router.map.internal.insert(addr.clone(), record);

    #[cfg(feature = "std")]
    if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
        trace!("{:#?}", router.map.internal);
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.map.internal);

    router.map.addr_map.insert(addr.clone(), addr.clone());

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
