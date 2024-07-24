use ockam_core::compat::time::Duration;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Route;
use ockam_core::{Error, Result};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// Type that [`UdpPuncture`] broadcasts
#[derive(Clone, Debug)]
pub enum UdpPunctureNotification {
    Open(Route),
    Closed,
}

pub async fn wait_for_puncture(
    receiver: &mut broadcast::Receiver<UdpPunctureNotification>,
    timeout: Duration,
) -> Result<Route> {
    tokio::time::timeout(timeout, async move {
        loop {
            match receiver.recv().await {
                Ok(notification) => match notification {
                    UdpPunctureNotification::Open(peer_route) => {
                        return Ok(peer_route);
                    }
                    UdpPunctureNotification::Closed => {
                        return Err(Error::new(
                            Origin::Transport,
                            Kind::Shutdown,
                            "UDP puncture was closed",
                        ))
                    }
                },
                Err(err) => match err {
                    RecvError::Closed => {
                        return Err(Error::new(
                            Origin::Transport,
                            Kind::Cancelled,
                            "UDP puncture won't be opened",
                        ))
                    }
                    RecvError::Lagged(_) => continue,
                },
            }
        }
    })
    .await
    .map_err(|_| {
        Error::new(
            Origin::Transport,
            Kind::Cancelled,
            format!("Timeout {:?} elapsed waiting for UDP puncture", timeout),
        )
    })?
}
