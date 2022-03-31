use super::Router;
use crate::tokio::sync::mpsc::Sender;
use crate::{error, NodeReply, NodeReplyResult};
use ockam_core::{Result, Address};

pub(super) async fn exec(
    router: &mut Router,
    addr: &Address,
    reply: &Sender<NodeReplyResult>,
) -> Result<()> {
    trace!("Stopping worker '{}'", addr);

    let primary_address = if let Some(p) = router.map.addr_map.get(addr) {
        p.clone()
    } else {
        reply
            .send(NodeReply::no_such_address(addr.clone()))
            .await
            .map_err(|e| error::node_internal(e))?;

        return Ok(());
    };

    let record = if let Some(r) = router.map.internal.remove(&primary_address) {
        r
    } else {
        // Actually should not happen
        reply
            .send(NodeReply::no_such_address(addr.clone()))
            .await
            .map_err(|e| error::node_internal(e))?;

        return Ok(());
    };

    for addr in record.address_set().iter() {
        router.map.addr_map.remove(addr);
    }

    reply
        .send(NodeReply::ok())
        .await
        .map_err(|e| error::node_internal(e))?;

    Ok(())
}
