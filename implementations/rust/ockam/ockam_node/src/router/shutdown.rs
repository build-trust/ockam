use super::Router;
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, Result};

/// Register a stop ACK
///
/// For every ACK we re-test whether the current cluster has stopped.
/// If not, we do nothing. If so, we trigger the next cluster to stop.
pub(super) async fn ack(router: &mut Router, addr: Address) -> Result<bool> {
    debug!("Handling shutdown ACK for {}", addr);

    // Permanently remove the address and corresponding worker
    router.map.free_address(addr);

    // If there are workers left in the cluster: keep waiting
    if !router.map.cluster_done() {
        return Ok(false);
    }

    // Check if there is a next cluster
    router.stop_next_cluster().await
}

impl Router {
    async fn stop_next_cluster(&mut self) -> Result<bool> {
        let next_cluster_addresses = self.map.next_cluster();

        match next_cluster_addresses {
            Some(vec) => {
                self.stop_cluster_addresses(vec).await?;
                Ok(false)
            }
            // If not, we are done!
            None => Ok(true),
        }
    }

    async fn stop_cluster_addresses(&mut self, addresses: Vec<Address>) -> Result<()> {
        let mut addrs = vec![];

        for address in addresses.iter() {
            if let Some(record) = self.map.get_address_record_mut(address) {
                record.stop().await?;
                if let Some(first_address) = record.address_set().first().cloned() {
                    addrs.push(first_address);
                } else {
                    error!("Empty Address Set during cluster stop");
                }
            }
        }

        addrs.into_iter().for_each(|addr| self.map.init_stop(addr));
        Ok(())
    }
}

/// Implement the graceful shutdown strategy
#[cfg_attr(not(feature = "std"), allow(unused_variables))]
pub(super) async fn graceful(
    router: &mut Router,
    timeout: u8,
    reply: SmallSender<NodeReplyResult>,
) -> Result<bool> {
    // Mark the router as shutting down to prevent spawning
    debug!("initiating graceful node shutdown");
    // This changes the router state to `Stopping`
    router.state.shutdown(reply);

    // Start by shutting down clusterless workers
    let mut cluster = vec![];
    for rec in router.map.non_cluster_workers().iter_mut() {
        if let Some(first_address) = rec.address_set().first().cloned() {
            debug!("stopping address {}", first_address);
            rec.stop().await?;
            cluster.push(first_address);
        } else {
            error!("empty address set during graceful shutdown");
        }
    }

    // If there _are_ no clusterless workers we go to the next cluster
    if cluster.is_empty() {
        return router.stop_next_cluster().await;
    }

    // Otherwise: keep track of addresses we are stopping
    cluster
        .into_iter()
        .for_each(|addr| router.map.init_stop(addr));

    // Start a timeout task to interrupt us...
    #[cfg(feature = "std")]
    {
        use crate::NodeMessage;
        use core::time::Duration;
        use tokio::{task, time};

        let sender = router.sender();
        let dur = Duration::from_secs(timeout as u64);
        task::spawn(async move {
            time::sleep(dur).await;
            warn!(%timeout, "shutdown timeout reached; aborting node!");
            // This works only because the state of the router is `Stopping`
            if sender.send(NodeMessage::AbortNode).await.is_err() {
                warn!("failed to send node abort signal to router");
            }
        });
    }

    info!("node was shutdown gracefully");

    // Return but DO NOT stop the router
    Ok(false)
}

/// Implement the immediate shutdown strategy
///
/// When triggering an `immediate` shutdown, all worker handles are
/// signaled to terminate, allowing workers to run their `async fn
/// shutdown(...)` hook.  However: the router will not wait for them!
/// Messages sent during the shutdown phase may not be delivered and
/// shutdown hooks may be suddenly interrupted by thread-deallocation.
pub(super) async fn immediate(
    router: &mut Router,
    reply: SmallSender<NodeReplyResult>,
) -> Result<()> {
    router.map.clear_address_records_map();
    router.state.kill();
    reply
        .send(RouterReply::ok())
        .await
        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
    Ok(())
}
