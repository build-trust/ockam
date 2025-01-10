use crate::puncture::rendezvous_service::{RendezvousRequest, RendezvousResponse};
use crate::{PunctureError, UdpBind};
use ockam_core::{Result, Route};
use ockam_node::{Context, MessageSendReceiveOptions};
use std::time::Duration;

// UDP and NAT Hole Punching are unreliable protocols. Expect send and receive
// failures and don't wait too long for them
const QUICK_TIMEOUT: Duration = Duration::from_secs(3);

/// Client to the Rendezvous server
pub struct RendezvousClient {
    rendezvous_route: Route,
}

impl RendezvousClient {
    /// Constructor
    pub fn new(udp_bind: &UdpBind, rendezvous_route: Route) -> Self {
        let full_route = udp_bind.sender_address().clone() + rendezvous_route;

        Self {
            rendezvous_route: full_route,
        }
    }

    /// Query the Rendezvous service
    pub async fn get_my_address(&self, ctx: &Context) -> Result<String> {
        let res = ctx
            .send_and_receive_extended::<RendezvousResponse>(
                self.rendezvous_route.clone(),
                RendezvousRequest::GetMyAddress,
                MessageSendReceiveOptions::new().with_timeout(QUICK_TIMEOUT),
            )
            .await?
            .into_body()?;

        let a = match res {
            RendezvousResponse::GetMyAddress(a) => a,
            _ => return Err(PunctureError::RendezvousResponseInvalidMessageType)?,
        };

        Ok(a)
    }

    /// Query the Rendezvous service
    pub async fn ping(&self, ctx: &Context) -> Result<()> {
        let res = ctx
            .send_and_receive_extended::<RendezvousResponse>(
                self.rendezvous_route.clone(),
                RendezvousRequest::Ping,
                MessageSendReceiveOptions::new().with_timeout(QUICK_TIMEOUT),
            )
            .await?
            .into_body()?;

        match res {
            RendezvousResponse::Pong => {}
            _ => return Err(PunctureError::RendezvousResponseInvalidMessageType)?,
        };

        Ok(())
    }
}
