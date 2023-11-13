use super::Router;
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

pub(super) async fn exec(
    router: &mut Router,
    addr: &Address,
    reply: &SmallSender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping processor '{}'", addr);

    // Resolve any secondary address to the primary address
    let primary_address = match router.map.get_primary_address(addr) {
        Some(p) => p.clone(),
        None => {
            reply
                .send(RouterReply::no_such_address(addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    // Get the internal address record
    let record = match router.map.get_address_record_mut(&primary_address) {
        Some(r) => r,
        None => {
            // Actually should not happen
            reply
                .send(RouterReply::no_such_address(addr.clone()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            return Ok(());
        }
    };

    // Then send processor shutdown signal
    record.stop().await?;

    // Signal back that everything went OK
    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    Ok(())
}
