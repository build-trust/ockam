use crate::{
    puncture::rendezvous_service::{RendezvousRequest, RendezvousResponse},
    UDP,
};
use ockam_core::{async_trait, Address, Result, Route, Routed, Worker};
use ockam_node::Context;
use tracing::{debug, info, warn};

/// Rendezvous Service allows other nodes to discover their public IP address and port via UDP.
///
/// The Rendezvous service is a part of UDP NAT Hole Punching (see [Wikipedia](https://en.wikipedia.org/wiki/UDP_hole_punching)).
///
/// # Example
///
/// ```rust
/// use ockam_transport_udp::{UdpTransport, UdpBindOptions, RendezvousService, UdpBindArguments};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
///
/// // Start a Rendezvous service with address 'rendezvous' and listen on UDP port 4000
/// RendezvousService::start(&ctx, "rendezvous")?;
/// let udp = UdpTransport::create(&ctx)?;
/// let bind = udp.bind(UdpBindArguments::new().with_bind_address("0.0.0.0:4000")?, UdpBindOptions::new()).await?;
/// ctx.flow_controls().add_consumer(&"rendezvous".into(), bind.flow_control_id());
/// # Ok(()) }
/// ```
pub struct RendezvousService;

impl RendezvousService {
    /// Start a new Rendezvous service with the given local address
    pub fn start(ctx: &Context, address: impl Into<Address>) -> Result<()> {
        ctx.start_worker(address.into(), RendezvousServiceWorker::new())
    }
}

/// Worker for the UDP Puncture Rendezvous service
struct RendezvousServiceWorker {}

impl RendezvousServiceWorker {
    fn new() -> Self {
        Self {}
    }

    /// Extract from `return route` everything just before we received the
    /// message via UDP
    fn parse_route(return_route: &Route) -> Route {
        let addrs = return_route
            .iter()
            .skip_while(|x| x.transport_type() != UDP)
            .cloned();

        let mut res = Route::new();
        for a in addrs {
            res = res.append(a);
        }
        res.into()
    }

    /// Extract first UDP Address from `return route`
    fn get_udp_address(return_route: &Route) -> Option<Address> {
        return_route
            .iter()
            .find(|&x| x.transport_type() == UDP)
            .cloned()
    }

    /// Handle Update request
    fn handle_get_my_address(&mut self, return_route: &Route) -> Option<String> {
        Self::get_udp_address(return_route).map(|a| a.address().to_string())
    }
}

#[async_trait]
impl Worker for RendezvousServiceWorker {
    type Message = RendezvousRequest;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        debug!(
            "Received message: {:?} from {}",
            msg,
            Self::parse_route(msg.return_route())
        );
        let return_route = msg.return_route().clone();
        match msg.into_body()? {
            RendezvousRequest::Ping => {
                ctx.send(return_route, RendezvousResponse::Pong).await?;
            }
            RendezvousRequest::GetMyAddress => {
                let res = self.handle_get_my_address(&return_route);
                match res {
                    Some(udp_address) => {
                        info!("{} got its public address", udp_address);
                        ctx.send(return_route, RendezvousResponse::GetMyAddress(udp_address))
                            .await?;
                    }
                    None => {
                        // This could happen if a client erroneously contacts this service over TCP not UDP, for example
                        warn!(
                            "Return route has no UDP part, will not return address map: {:?}",
                            return_route
                        );
                        // Ignore issue. There's no (current) way to inform sender.
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RendezvousServiceWorker;
    use crate::puncture::rendezvous_service::{RendezvousRequest, RendezvousResponse};
    use crate::{RendezvousService, UdpBind, UdpBindArguments, UdpBindOptions, UdpTransport, UDP};
    use ockam_core::{route, Result, Route, TransportType};
    use ockam_node::Context;

    #[test]
    fn parse_route() {
        assert_eq!(
            RendezvousServiceWorker::parse_route(&route!["a", "b", (UDP, "c")]),
            route![(UDP, "c")]
        );
        assert_eq!(
            RendezvousServiceWorker::parse_route(&route!["a", "b", (UDP, "c"), "d"]),
            route![(UDP, "c"), "d"]
        );
        assert_eq!(
            RendezvousServiceWorker::parse_route(&route!["a", "b", (UDP, "c"), "d", "e"]),
            route![(UDP, "c"), "d", "e"]
        );
        let not_udp = TransportType::new((u8::from(UDP)) + 1);
        assert_eq!(
            RendezvousServiceWorker::parse_route(&route!["a", "b", (not_udp, "c"), "d"]),
            route![]
        );
        assert_eq!(
            RendezvousServiceWorker::parse_route(&route!["a", "b", "c", "d"]),
            route![]
        );
        assert_eq!(RendezvousServiceWorker::parse_route(&route![]), route![]);
    }

    #[ockam_macros::test]
    async fn ping(ctx: &mut Context) -> Result<()> {
        let (rendezvous_route, _) = test_setup(ctx).await?;

        let res: RendezvousResponse = ctx
            .send_and_receive(rendezvous_route, RendezvousRequest::Ping)
            .await?;
        assert!(matches!(res, RendezvousResponse::Pong));
        Ok(())
    }

    #[ockam_macros::test]
    async fn get_my_address(ctx: &mut Context) -> Result<()> {
        let (rendezvous_route, udp_bind) = test_setup(ctx).await?;

        let res: RendezvousResponse = ctx
            .send_and_receive(rendezvous_route, RendezvousRequest::GetMyAddress)
            .await?;

        match res {
            RendezvousResponse::GetMyAddress(address) => {
                assert_eq!(address, udp_bind.bind_address().to_string());
            }
            _ => panic!(),
        }

        Ok(())
    }

    /// Helper
    async fn test_setup(ctx: &mut Context) -> Result<(Route, UdpBind)> {
        // Create transport, start rendezvous service, start echo service and listen
        let transport = UdpTransport::create(ctx)?;
        RendezvousService::start(ctx, "rendezvous")?;

        let udp_bind = transport
            .bind(UdpBindArguments::new(), UdpBindOptions::new())
            .await?;

        ctx.flow_controls()
            .add_consumer(&"rendezvous".into(), udp_bind.flow_control_id());

        let bind_addr = udp_bind.bind_address().to_string();

        let rendezvous_route = route![
            udp_bind.sender_address().clone(),
            (UDP, bind_addr.to_string()),
            "rendezvous"
        ];

        ctx.flow_controls()
            .add_consumer(&"echo".into(), udp_bind.flow_control_id());

        Ok((rendezvous_route, udp_bind))
    }
}
