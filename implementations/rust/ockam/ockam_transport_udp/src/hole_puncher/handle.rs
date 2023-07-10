use super::message::PunchMessage;
use crate::{hole_puncher::worker::UdpHolePunchWorker, PunchError};
use ockam_core::{Address, AllowOnwardAddress, AllowSourceAddress, Result, Route};
use ockam_node::Context;

/// High level management interface for UDP NAT Hole Punchers
///
/// See [Wikipedia](https://en.wikipedia.org/wiki/UDP_hole_punching) and
/// ['Peer-to-Peer Communication Across Network Address Translators'](https://bford.info/pub/net/p2pnat/).
///
/// A node can have multiple Hole Punchers (punchers) at a time, each one with
/// a unique name and working with a different peer puncher in a remote node.
///
/// For a puncher to work a (e.g. from 'alice' to 'bob', using rendezvous service
/// 'zurg') the remote node will also need to create its own puncher
/// (e.g. from 'bob' to 'alice', using 'zurg').
///
/// # Warnings
///
/// This UDP NAT Hole Puncher implementation is __currently a prototype__.
/// No guarantees are provided.
///
/// UDP and NAT Hole Punching are unreliable protocols. Expect send and receive
/// failures.
///
/// # Example
///
/// ```rust
/// # use {ockam_node::Context, ockam_core::{Result, route}};
/// # async fn test(ctx: &mut Context) -> Result<()> {
/// use ockam_transport_udp::{UdpHolePuncher, UdpTransport, UDP};
///
/// // Create transport
/// UdpTransport::create(ctx).await?;
///
/// // Create a NAT hole from us 'alice' to them 'bob' using
/// // the Rendezvous service 'zurg' at public IP address `192.168.1.10:4000`
/// let rendezvous_route = route![(UDP, "192.168.1.10:4000"), "zurg"];
/// let mut puncher = UdpHolePuncher::create(ctx, "alice", "bob", rendezvous_route).await?;
///
/// // Note: For this to work, 'bob' will likewise need to create a hole thru to us
///
/// // Wait for hole to open.
/// // Note that the hole could close at anytime. If the hole closes, the
/// // puncher will automatically try to re-open it.
/// puncher.wait_for_hole_open().await?;
///
/// // Try to send a message to a remote 'echoer' via our puncher
/// ctx.send(route![puncher.address(), "echoer"], "Góðan daginn".to_string()).await?;
/// # Ok(())
/// # }
/// ```
///
pub struct UdpHolePuncher {
    ctx: Context,
    worker_main_addr: Address,
    worker_local_addr: Address,
}

// TODO: Allow app to specify how often keepalives are used - they may have
// limited bandwidth. Also, allow app to specify other configurations?

impl UdpHolePuncher {
    /// Create a new UDP NAT Hole Puncher
    pub async fn create<S: AsRef<str>, R: Into<Route>>(
        ctx: &mut Context,
        puncher_name: S,
        peer_puncher_name: S,
        rendezvous_route: R,
    ) -> Result<UdpHolePuncher> {
        // Check if we can reach the rendezvous service
        let rendezvous_route = rendezvous_route.into();

        if !UdpHolePunchWorker::rendezvous_reachable(ctx, &rendezvous_route).await {
            return Err(PunchError::RendezvousServiceNotFound.into());
        }

        // Create worker
        let handle_addr = Address::random_tagged("UdpHolePuncher.detached");
        let (worker_main_addr, worker_local_addr) = UdpHolePunchWorker::create(
            ctx,
            &handle_addr,
            rendezvous_route,
            puncher_name.as_ref(),
            peer_puncher_name.as_ref(),
        )
        .await?;

        // Handle has a context for messaging the `UdpHolePunchWorker`
        let handle_ctx = ctx
            .new_detached(
                handle_addr,
                AllowSourceAddress(worker_main_addr.clone()),
                AllowOnwardAddress(worker_main_addr.clone()),
            )
            .await?;

        Ok(Self {
            ctx: handle_ctx,
            worker_main_addr,
            worker_local_addr,
        })
    }

    /// Wait until Hole Puncher successfully opens a hole to the peer or a
    /// timeout
    ///
    /// Note that the hole could close at anytime. If the hole closes, the
    /// puncher will automatically try to re-open it.
    ///
    /// Timeout is the same as that of [`Context::receive()`].
    pub async fn wait_for_hole_open(&mut self) -> Result<()> {
        self.ctx
            .send(self.worker_main_addr.clone(), PunchMessage::WaitForHoleOpen)
            .await?;
        self.ctx.receive::<()>().await?;
        Ok(())
    }

    /// Address of this UDP NAT Hole Puncher's worker.
    pub fn address(&self) -> Address {
        self.worker_local_addr.clone()
    }
}
