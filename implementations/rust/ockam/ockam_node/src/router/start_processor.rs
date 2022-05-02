use super::{AddressMeta, AddressRecord, NodeState, Router, SenderPair};
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

/// Execute a `StartWorker` command
#[tracing::instrument(name = "start_processor", skip_all, err, fields(addrs = ?addrs))]
pub(super) async fn exec(
    router: &mut Router,
    addrs: Address,
    senders: SenderPair,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, senders, reply).await,
        NodeState::Stopping(_) => reject(reply).await,
        NodeState::Dead => unreachable!(),
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addr: Address,
    senders: SenderPair,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    debug!("Starting new processor '{}'", &addr);
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
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
    Ok(())
}

#[tracing::instrument(name = "start_processor::reject", skip_all, err)]
async fn reject(reply: &Sender<NodeReplyResult>) -> Result<()> {
    trace!("StartWorker command rejected: node shutting down");
    reply
        .send(RouterReply::node_rejected(NodeReason::Shutdown))
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
    Ok(())
}
