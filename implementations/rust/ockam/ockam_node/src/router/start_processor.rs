use super::{AddressRecord, NodeState, Router, SenderPair, WorkerMeta};
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    AddressMetadata, NodeReplyResult, RouterReason, RouterReply,
};
use ockam_core::compat::{sync::Arc, vec::Vec};
#[cfg(feature = "std")]
use ockam_core::env::get_env;
use ockam_core::{Address, Result};

/// Execute a `StartWorker` command
pub(super) async fn exec(
    router: &mut Router,
    addrs: Vec<Address>,
    senders: SenderPair,
    addresses_metadata: Vec<AddressMetadata>,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    match router.state.node_state() {
        NodeState::Running => start(router, addrs, senders, addresses_metadata, reply).await,
        NodeState::Stopping(_) => reject(reply).await,
        NodeState::Dead => unreachable!(),
    }?;
    Ok(())
}

async fn start(
    router: &mut Router,
    addrs: Vec<Address>,
    senders: SenderPair,
    addresses_metadata: Vec<AddressMetadata>,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    let primary_addr = addrs
        .first()
        .ok_or_else(|| NodeError::RouterState(RouterReason::EmptyAddressSet).internal())?;

    router.check_addr_not_exist(primary_addr, reply).await?;

    debug!("Starting new processor '{}'", &primary_addr);

    let SenderPair { msgs, ctrl } = senders;

    let record = AddressRecord::new(
        addrs.clone(),
        msgs,
        ctrl,
        // We don't keep track of the mailbox count for processors
        // because, while they are able to send and receive messages
        // via their mailbox, most likely this metric is going to be
        // irrelevant.  We may want to re-visit this decision in the
        // future, if the way processors are used changes.
        Arc::new(0.into()),
        WorkerMeta {
            processor: true,
            detached: false,
        },
    );

    router
        .map
        .insert_address_record(primary_addr.clone(), record);

    for metadata in addresses_metadata {
        if !addrs.contains(&metadata.address) {
            warn!(
                "Address {} is not in the set of addresses for this processor",
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
    if let Ok(Some(dump_internals)) = get_env::<bool>("OCKAM_DUMP_INTERNALS") {
        if dump_internals {
            trace!("{:#?}", router.map.address_records_map());
        }
    }
    #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
    trace!("{:#?}", router.map.address_records_map());

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
    trace!("StartProcessor command rejected: node shutting down");
    reply
        .send(RouterReply::node_rejected(NodeReason::Shutdown))
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
    Ok(())
}
