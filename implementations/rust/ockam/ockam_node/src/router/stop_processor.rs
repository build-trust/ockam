use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::{Address, Result};

#[tracing::instrument(name = "stop_processor", skip_all, err, fields(addr = ?main_addr))]
pub(super) async fn exec(
    router: &mut Router,
    main_addr: &Address,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping processor '{}'", main_addr);

    // First check if the processor exists
    let mut record = match router.map.internal.remove(main_addr) {
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
    router.map.addr_map.remove(main_addr);

    // Then send processor shutdown signal
    record.stop().await?;

    // Signal back that everything went OK
    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

    Ok(())
}
