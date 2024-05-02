use crate::puncture::rendezvous_service::{RendezvousRequest, RendezvousResponse};
use crate::{PunctureError, UdpBind};
use ockam_core::AsyncTryClone;
use ockam_core::{route, Address, AllowAll, Result, Route};
use ockam_node::{Context, MessageSendReceiveOptions};
use std::time::Duration;

// UDP and NAT Hole Punching are unreliable protocols. Expect send and receive
// failures and don't wait too long for them
const QUICK_TIMEOUT: Duration = Duration::from_secs(3);

/// Client to the Rendezvous server
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct RendezvousClient {
    ctx: Context,
    rendezvous_route: Route,
}

impl RendezvousClient {
    /// Constructor
    pub async fn new(ctx: &Context, udp_bind: &UdpBind, rendezvous_route: Route) -> Result<Self> {
        let ctx = ctx
            .new_detached(
                Address::random_tagged("RendezvousClient"),
                AllowAll,
                AllowAll,
            )
            .await?;

        let full_route = route![udp_bind.sender_address().clone(), rendezvous_route];

        Ok(Self {
            ctx,
            rendezvous_route: full_route,
        })
    }

    /// Query the Rendezvous service
    pub async fn get_my_address(&self) -> Result<String> {
        let res = self
            .ctx
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
}
