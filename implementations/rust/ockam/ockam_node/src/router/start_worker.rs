use super::{AddressRecord, NodeState, Router, SenderPair, WorkerMeta};
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    AddressMetadata, NodeReplyResult, RouterReason, RouterReply,
};
use core::sync::atomic::AtomicUsize;
#[cfg(feature = "std")]
use ockam_core::env::get_env;
use ockam_core::{
    compat::{sync::Arc, vec::Vec},
    Address, Result,
};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: Vec<Address>,
    senders: SenderPair,
    detached: bool,
    addresses_metadata: Vec<AddressMetadata>,
    metrics: Arc<AtomicUsize>,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => {
            start(
                router,
                addrs,
                senders,
                detached,
                addresses_metadata,
                metrics,
                reply,
            )
            .await
        }
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
    addresses_metadata: Vec<AddressMetadata>,
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
        WorkerMeta {
            processor: false,
            detached,
        },
    );

    router
        .map
        .insert_address_record(primary_addr.clone(), address_record);

    for metadata in addresses_metadata {
        if !addrs.contains(&metadata.address) {
            warn!(
                "Address {} is not in the set of addresses for this worker",
                metadata.address
            );
            continue;
        }

        if metadata.is_terminal {
            router
                .map
                .mark_address_as_terminal(metadata.address.clone());
        }

        for (key, value) in metadata.attributes {
            router
                .map
                .write_address_metadata(metadata.address.clone(), &key, &value);
        }
    }

    #[cfg(feature = "std")]
    if let Ok(Some(_)) = get_env::<String>("OCKAM_DUMP_INTERNALS") {
        trace!("{:#?}", router.map.address_records_map());
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.map.internal);

    addrs.iter().for_each(|addr| {
        router.map.insert_alias(addr, primary_addr);
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
