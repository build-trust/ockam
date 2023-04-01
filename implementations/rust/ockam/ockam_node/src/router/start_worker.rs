use super::{AddressMeta, AddressRecord, NodeState, Router, SenderPair};
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReason, RouterReply,
};
use core::sync::atomic::AtomicUsize;
use ockam_core::{
    compat::{sync::Arc, vec::Vec},
    env::get_env,
    Address, Result,
};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: Vec<Address>,
    senders: SenderPair,
    detached: bool,
    metrics: Arc<AtomicUsize>,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, senders, detached, metrics, reply).await,
        NodeState::Stopping(_) => reject(reply).await,
        NodeState::Dead => unreachable!(),
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addrs: Vec<Address>,
    senders: SenderPair,
    detached: bool,
    metrics: Arc<AtomicUsize>,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    let primary_addr = addrs
        .first()
        .ok_or_else(|| NodeError::RouterState(RouterReason::EmptyAddressSet).internal())?;

    router.check_addr_not_exist(primary_addr, reply).await?;

    debug!("Starting new worker '{}'", primary_addr);

    let SenderPair { msgs, ctrl } = senders;

    // Create an address record and insert it into the internal map

    // FIXME: Check for duplicates
    let address_record = AddressRecord::new(
        addrs.clone(),
        msgs,
        ctrl,
        metrics,
        AddressMeta {
            processor: false,
            detached,
        },
    );

    router
        .map
        .internal
        .insert(primary_addr.clone(), address_record);

    #[cfg(feature = "std")]
    if let Ok(Some(_)) = get_env::<String>("OCKAM_DUMP_INTERNALS") {
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
