use super::{AddressMeta, AddressRecord, NodeState, Router, SenderPair};
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{compat::sync::Arc, Address, Result};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: Address,
    senders: SenderPair,
    reply: &SmallSender<NodeReplyResult>,
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
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    router.check_addr_not_exist(&addr, reply).await?;

    debug!("Starting new processor '{}'", &addr);

    let SenderPair { msgs, ctrl } = senders;

    let record = AddressRecord::new(
        vec![addr.clone()],
        msgs,
        ctrl,
        // We don't keep track of the mailbox count for processors
        // because, while they are able to send and receive messages
        // via their mailbox, most likely this metric is going to be
        // irrelevant.  We may want to re-visit this decision in the
        // future, if the way processors are used changes.
        Arc::new(0.into()),
        AddressMeta {
            processor: true,
            detached: false,
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

async fn reject(reply: &SmallSender<NodeReplyResult>) -> Result<()> {
    trace!("StartWorker command rejected: node shutting down");
    reply
        .send(RouterReply::node_rejected(NodeReason::Shutdown))
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
    Ok(())
}
