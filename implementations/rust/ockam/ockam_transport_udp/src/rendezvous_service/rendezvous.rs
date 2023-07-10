use crate::{
    rendezvous_service::{RendezvousRequest, RendezvousResponse},
    UDP,
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, Error, Result, Route, Routed, Worker};
use ockam_node::Context;
use std::collections::BTreeMap;
use tracing::{debug, trace, warn};

/// High level management interface for UDP Rendezvous Service
///
/// The Rendezvous service is a part of UDP NAT Hole Punching (see [Wikipedia](https://en.wikipedia.org/wiki/UDP_hole_punching)).
///
/// A node could start multiple Rendezvous services, each with its own address.
///
/// To work, this service requires the UDP Transport to be working.
///
/// # Example
///
/// ```rust
/// use ockam_transport_udp::{UdpTransport, UdpRendezvousService};
/// # use ockam_node::Context;
/// # use ockam_core::Result;
/// # async fn test(ctx: Context) -> Result<()> {
///
/// // Start a Rendezvous service with address 'my_rendezvous' and listen on UDP port 4000
/// UdpRendezvousService::start(&ctx, "my_rendezvous").await?;
/// let udp = UdpTransport::create(&ctx).await?;
/// udp.listen("0.0.0.0:4000").await?;
/// # Ok(()) }
/// ```
pub struct UdpRendezvousService;

impl UdpRendezvousService {
    /// Start a new Rendezvous service with the given local address
    pub async fn start(ctx: &Context, address: impl Into<Address>) -> Result<()> {
        ctx.start_worker(address.into(), RendezvousWorker::new())
            .await
    }
}

// TODO: Implement mechanism for removing entries from internal map, possibly by
// deleting 'old' entries on heartbeat events.

/// Worker for the UDP NAT Hole Punching Rendezvous service
///
/// Maintains an internal map for remote nodes and the public IP address
/// from which they send UDP datagrams.
///
/// Remote nodes can send requests to update and query the map.
struct RendezvousWorker {
    map: BTreeMap<String, Route>,
}

impl Default for RendezvousWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl RendezvousWorker {
    fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
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

    // Handle Update request
    fn handle_update(&mut self, puncher_name: &str, return_route: &Route) {
        let r = Self::parse_route(return_route);
        if !r.is_empty() {
            self.map.insert(puncher_name.to_owned(), r);
        } else {
            // This could happen if a client erroneously contacts this service over TCP not UDP, for example
            warn!(
                "Return route has no UDP part, will not update map: {:?}",
                return_route
            );
            // Ignore issue. There's no (current) way to inform sender.
        }
    }

    // Handle Query request
    fn handle_query(&self, puncher_name: &String) -> Result<Route> {
        match self.map.get(puncher_name) {
            Some(route) => Ok(route.clone()),
            None => Err(Error::new_without_cause(Origin::Other, Kind::NotFound)),
        }
    }
}

#[async_trait]
impl Worker for RendezvousWorker {
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
            Self::parse_route(&msg.return_route())
        );
        let return_route = msg.return_route();
        match msg.as_body() {
            RendezvousRequest::Update { puncher_name } => {
                self.handle_update(puncher_name, &return_route);
            }
            RendezvousRequest::Query { puncher_name } => {
                let res = self.handle_query(puncher_name);
                ctx.send(return_route, RendezvousResponse::Query(res))
                    .await?;
            }
            RendezvousRequest::Ping => {
                ctx.send(return_route, RendezvousResponse::Pong).await?;
            }
        }
        trace!("Map: {:?}", self.map);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RendezvousWorker;
    use crate::rendezvous_service::{RendezvousRequest, RendezvousResponse};
    use crate::{UdpRendezvousService, UdpTransport, UDP};
    use ockam_core::errcode::Origin;
    use ockam_core::{route, Error, Result, Route, Routed, TransportType, Worker};
    use ockam_node::Context;
    use std::net::SocketAddr;
    use tokio::net::UdpSocket;
    use tracing::debug;

    #[test]
    fn parse_route() {
        assert_eq!(
            route![(UDP, "c")],
            RendezvousWorker::parse_route(&route!["a", "b", (UDP, "c")])
        );
        assert_eq!(
            route![(UDP, "c"), "d"],
            RendezvousWorker::parse_route(&route!["a", "b", (UDP, "c"), "d"])
        );
        assert_eq!(
            route![(UDP, "c"), "d", "e"],
            RendezvousWorker::parse_route(&route!["a", "b", (UDP, "c"), "d", "e"])
        );
        let not_udp = TransportType::new((u8::from(UDP)) + 1);
        assert_eq!(
            route![],
            RendezvousWorker::parse_route(&route!["a", "b", (not_udp, "c"), "d"])
        );
        assert_eq!(
            route![],
            RendezvousWorker::parse_route(&route!["a", "b", "c", "d"])
        );
        assert_eq!(route![], RendezvousWorker::parse_route(&route![]));
    }

    #[ockam_macros::test]
    async fn update_and_query(ctx: &mut Context) -> Result<()> {
        let (rendezvous_route, send_addr) = test_setup(ctx).await?;

        let our_public_route = route![(UDP, send_addr.to_string()), ctx.address()];

        // Update service, should work
        //
        // Use Alice and Bob with the same address to check the service can
        // handle multiple internal mappings and that multiple map values
        // can be for the same node.
        update_operation("Alice", ctx, &rendezvous_route)
            .await
            .unwrap();
        update_operation("Bob", ctx, &rendezvous_route)
            .await
            .unwrap();

        // Query service, should work
        let res = query_operation("Alice", ctx, &rendezvous_route)
            .await
            .unwrap();
        assert_eq!(res, our_public_route);
        let res = query_operation("Bob", ctx, &rendezvous_route)
            .await
            .unwrap();
        assert_eq!(res, our_public_route);

        // Query service for non-existent node, should error
        let res = query_operation("DoesNotExist", ctx, &rendezvous_route).await;
        assert!(res.is_err(), "Query operation should have failed");

        // Shutdown
        ctx.stop().await?;
        Ok(())
    }

    #[ockam_macros::test]
    async fn ping(ctx: &mut Context) -> Result<()> {
        let (rendezvous_route, _) = test_setup(ctx).await?;

        let res: RendezvousResponse = ctx
            .send_and_receive(rendezvous_route, RendezvousRequest::Ping)
            .await?;
        assert!(matches!(res, RendezvousResponse::Pong));

        // Shutdown
        ctx.stop().await?;
        Ok(())
    }

    /// Helper
    async fn test_setup(ctx: &mut Context) -> Result<(Route, SocketAddr)> {
        // Find an available port
        let bind_addr = *available_local_ports(1).await?.first().unwrap();
        debug!("bind_addr = {:?}", bind_addr);

        // Create transport, start rendezvous service, start echo service and listen
        let transport = UdpTransport::create(ctx).await?;
        UdpRendezvousService::start(ctx, "rendezvous").await?;
        let rendezvous_route = route![(UDP, bind_addr.to_string()), "rendezvous"];
        ctx.start_worker("echo", EchoUDPAddress).await?;
        let route_echo = route![(UDP, bind_addr.to_string()), "echo"];
        transport.listen(bind_addr.to_string()).await?;

        // Use echo service to find out our UDP sending address
        let send_addr: String = ctx.send_and_receive(route_echo, String::new()).await?;
        let send_addr = send_addr.parse::<SocketAddr>().unwrap();

        Ok((rendezvous_route, send_addr))
    }

    /// Helper
    async fn update_operation(puncher_name: &str, ctx: &mut Context, route: &Route) -> Result<()> {
        let msg = RendezvousRequest::Update {
            puncher_name: String::from(puncher_name),
        };

        // Send from our context's main address
        ctx.send(route.clone(), msg).await
    }

    /// Helper
    async fn query_operation(puncher_name: &str, ctx: &Context, route: &Route) -> Result<Route> {
        let msg = RendezvousRequest::Query {
            puncher_name: String::from(puncher_name),
        };
        let res: RendezvousResponse = ctx.send_and_receive(route.clone(), msg).await?;
        match res {
            RendezvousResponse::Query(r) => r,
            r => panic!("Unexpected response: {:?}", r),
        }
    }

    /// Echo service that allows us to find out the UDP address the tests are sending from
    struct EchoUDPAddress;

    #[ockam_core::worker]
    impl Worker for EchoUDPAddress {
        type Message = String;
        type Context = Context;

        async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
            // Get source UDP address
            let src_addr = match msg
                .return_route()
                .iter()
                .find(|x| x.transport_type() == UDP)
            {
                Some(addr) => String::from(addr.address()),
                None => panic!("Return route has no UDP hop"),
            };

            // Reply
            debug!("Replying '{}' to {}", src_addr, &msg.return_route());
            ctx.send(msg.return_route(), src_addr).await
        }
    }

    const AVAILABLE_LOCAL_PORTS_ADDR: &str = "127.0.0.1:0";

    /// Helper function. Try to find numbers of available local UDP ports.
    async fn available_local_ports(count: usize) -> Result<Vec<SocketAddr>> {
        let mut sockets = Vec::new();
        let mut addrs = Vec::new();

        for _ in 0..count {
            let s = UdpSocket::bind(AVAILABLE_LOCAL_PORTS_ADDR)
                .await
                .map_err(|e| Error::new_unknown(Origin::Unknown, e))?;
            let a = s
                .local_addr()
                .map_err(|e| Error::new_unknown(Origin::Unknown, e))?;

            addrs.push(a);

            // Keep sockets open until we are done asking for available ports
            sockets.push(s);
        }

        Ok(addrs)
    }
}
