use super::Router;
use crate::channel_types::SmallSender;
use crate::{
    error::{NodeError, NodeReason},
    NodeReplyResult, RouterReply,
};
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
    stop_next_cluster(router).await
}

async fn stop_next_cluster(r: &mut Router) -> Result<bool> {
    match r.map.next_cluster() {
        Some(mut vec) => {
            let mut addrs = vec![];

            for record in vec.iter_mut() {
                record.stop().await?;
                if let Some(first_address) = record.address_set().first().cloned() {
                    addrs.push(first_address);
                } else {
                    error!("Empty Address Set during cluster stop");
                }
            }

            addrs.into_iter().for_each(|addr| r.map.init_stop(addr));
            Ok(false)
        }
        // If not, we are done!
        None => Ok(true),
    }
}

/// Implement the graceful shutdown strategy
#[cfg_attr(not(feature = "std"), allow(unused_variables))]
pub(super) async fn graceful(
    router: &mut Router,
    seconds: u8,
    reply: SmallSender<NodeReplyResult>,
) -> Result<bool> {
    // Mark the router as shutting down to prevent spawning
    info!("Initiate graceful node shutdown");
    // This changes the router state to `Stopping`
    router.state.shutdown(reply);

    // Start by shutting down clusterless workers
    let mut cluster = vec![];
    for rec in router.map.non_cluster_workers().iter_mut() {
        if let Some(first_address) = rec.address_set().first().cloned() {
            debug!("Stopping address {}", first_address);
            rec.stop().await?;
            cluster.push(first_address);
        } else {
            error!("Empty Address Set during graceful shutdown");
        }
    }

    // If there _are_ no clusterless workers we go to the next cluster
    if cluster.is_empty() {
        return stop_next_cluster(router).await;
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
        let dur = Duration::from_secs(seconds as u64);
        task::spawn(async move {
            time::sleep(dur).await;
            warn!("Shutdown timeout reached; aborting node!");
            // This works only because the state of the router is `Stopping`
            if sender.send(NodeMessage::AbortNode).await.is_err() {
                error!("Failed to send node abort signal to router");
            }
        });
    }

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
