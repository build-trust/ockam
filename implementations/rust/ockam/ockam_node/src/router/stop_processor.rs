use super::Router;
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    main_addr: &Address,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping processor '{}'", main_addr);

    // First check if the processor exists
    let mut record = match router.map.remove_address_record(main_addr) {
        Some(proc) => proc,
        None => {
            reply
                .send(RouterReply::no_such_address(main_addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    // Remove  main address from addr_map too
    router.map.remove_alias(main_addr);

    // Then send processor shutdown signal
    record.stop().await?;

    // Signal back that everything went OK
    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    Ok(())
}
