use super::Router;
use crate::{
    error::Error,
    tokio::{sync::mpsc::Sender, task, time},
    NodeMessage, NodeReply, NodeReplyResult,
};
use core::time::Duration;
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
                addrs.push(record.address_set().first().clone());
            }

            addrs.into_iter().for_each(|addr| r.map.init_stop(addr));
            Ok(false)
        }
        // If not, we are done!
        None => Ok(true),
    }
}

/// Implement the graceful shutdown strategy
pub(super) async fn graceful(
    router: &mut Router,
    seconds: u8,
    reply: Sender<NodeReplyResult>,
) -> Result<bool> {
    // Mark the router as shutting down to prevent spawning
    info!("Initiate graceful node shutdown");
    router.state.shutdown(reply);

    // Start by shutting down clusterless workers
    let mut cluster = vec![];
    for rec in router.map.non_cluster_workers().iter_mut() {
        debug!("Stopping address {}", rec.address_set().first());
        rec.stop().await?;
        cluster.push(rec.address_set().first());
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
    let sender = router.sender();
    let dur = Duration::from_secs(seconds as u64);
    task::spawn(async move {
        time::sleep(dur).await;
        warn!("Shutdown timeout reached; aborting node!");
        if sender.send(NodeMessage::AbortNode).await.is_err() {
            error!("Failed to send node abort signal to router");
        }
    });

    // Return but DO NOT stop the router
    Ok(false)
}

/// Implement the immediate shutdown strategy
///
/// When triggering an `immediate` shutdown, all worker handles are
/// signalled to terminate, allowing workers to run their `async fn
/// shutdown(...)` hook.  However: the router will not wait for them!
/// Messages sent during the shutdown phase may not be delivered and
/// shutdown hooks may be suddenly interrupted by thread-deallocation.
pub(super) async fn immediate(router: &mut Router, reply: Sender<NodeReplyResult>) -> Result<()> {
    router.map.internal.clear();
    reply
        .send(NodeReply::ok())
        .await
        .map_err(|_| Error::InternalIOFailure)?;
    Ok(())
}
