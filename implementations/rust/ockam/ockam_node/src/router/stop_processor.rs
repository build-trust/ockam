use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{error, NodeReply, NodeReplyResult};
use ockam_core::{error::Result, Address};

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
                .send(NodeReply::no_such_address(main_addr.clone()))
                .await
                .map_err(|e| error::node_internal(e))?;

            return Ok(());
        }
    };

    // Remove  main address from addr_map too
    router.map.addr_map.remove(main_addr);

    // Then send processor shutdown signal
    record.stop().await?;

    // Signal back that everything went OK
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|e| error::node_internal(e))?;

    Ok(())
}
