use super::{Router, RouterState};
use crate::tokio::time;
use crate::WorkerShutdownPriority;
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

impl Router {
    /// Implement the graceful shutdown strategy
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
    pub async fn shutdown_graceful(self: Arc<Router>, seconds: u8) -> Result<()> {
        // This changes the router state to `Stopping`
        let state = {
            let mut state = self.state.write().unwrap();

            let state_val = *state;
            if state_val == RouterState::Running {
                *state = RouterState::ShuttingDown;
            }

            state_val
        };

        match state {
            RouterState::Running => {}
            RouterState::ShuttingDown => {
                info!("Router is already stopping");
                self.wait_termination().await;
                return Ok(());
            }
            RouterState::Shutdown => {
                info!("Router is already stopped");
                return Ok(());
            }
        }

        info!("Initiate graceful node shutdown");

        // Start a timeout task to interrupt us...
        let dur = Duration::from_secs(seconds as u64);

        let r = self.clone();
        let timeout = async move {
            time::sleep(dur).await;

            // TODO: This actually doesn't abort anything, but it should unblock the .stop call, so
            //  that we can process and eventually drop the tokio Runtime
            warn!("Shutdown timeout reached; aborting node!");
            let uncleared_addresses = r.map.force_clear_records();

            if !uncleared_addresses.is_empty() {
                error!(
                    "Router internal inconsistency detected.\
                     Records map is not empty after stopping all workers. Addresses: {:?}",
                    uncleared_addresses
                );
            }
        };

        let r = self.clone();
        let shutdown = async move {
            for shutdown_priority in WorkerShutdownPriority::all_descending_order() {
                debug!("Stopping workers with priority: {:?}", shutdown_priority);
                let shutdown_yield_receiver = r.map.stop_workers(shutdown_priority);

                if let Some(shutdown_yield_receiver) = shutdown_yield_receiver {
                    debug!(
                        "Waiting for yield for workers with priority: {:?}",
                        shutdown_priority
                    );
                    // Wait for stop ack
                    match shutdown_yield_receiver.await {
                        Ok(_) => {
                            debug!(
                                "Received yield for workers with priority: {:?}",
                                shutdown_priority
                            );
                        }
                        Err(err) => {
                            error!("Error receiving shutdown yield: {}", err);
                        }
                    }
                } else {
                    debug!(
                        "There was no workers with priority: {:?}",
                        shutdown_priority
                    );
                }
            }

            debug!("Router shutdown finished");
        };

        #[cfg(feature = "std")]
        crate::tokio::select! {
            _ = shutdown => {}
            _ = timeout => {}
        }

        #[cfg(not(feature = "std"))]
        shutdown.await;

        debug!("Setting Router state to Shutdown");
        *self.state.write().unwrap() = RouterState::Shutdown;
        debug!("Sending Router shutdown broadcast");
        #[cfg(feature = "std")]
        match self.shutdown_broadcast_sender.write().unwrap().take() {
            None => {
                warn!("Couldn't send Router shutdown message. Channel is missing.");
            }
            Some(shutdown_broadcast_sender) => {
                if shutdown_broadcast_sender.send(()).is_err() {
                    // That's fine, it's possible nobody is listening for that broadcast
                    debug!("Couldn't send Router shutdown message. Sending error.");
                }
            }
        }

        info!("No more workers left. Goodbye!");

        Ok(())
    }
}
