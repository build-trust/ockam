use std::net::SocketAddr;

use ockam_core::{route, AllowAll, Result, Route, Routed, Worker};
use ockam_node::Context;
use ockam_transport_udp::{
    RendezvousRequest, RendezvousResponse, RendezvousWorker, UdpTransport, UDP,
};
use tracing::debug;

mod utils;

/// Test the Rendezvous service
#[ockam_macros::test]
async fn rendezvous_service(ctx: &mut Context) -> Result<()> {
    // Find an available port
    let bind_addr = *utils::available_local_ports(1).await?.first().unwrap();
    debug!("bind_addr = {:?}", bind_addr);

    // Create transport, start rendezvous service, start echo service and listen
    let transport = UdpTransport::create(ctx).await?;
    ctx.start_worker("rendezvous", RendezvousWorker::new(), AllowAll, AllowAll)
        .await?;
    let route_rendezvous = route![(UDP, bind_addr.to_string()), "rendezvous"];
    ctx.start_worker("echo", EchoUDPAddress, AllowAll, AllowAll)
        .await?;
    let route_echo = route![(UDP, bind_addr.to_string()), "echo"];
    transport.listen(bind_addr.to_string()).await?;

    // Use echo service to find out our UDP sending address
    let send_addr: String = ctx.send_and_receive(route_echo, String::new()).await?;
    let send_addr = send_addr.parse::<SocketAddr>().unwrap();

    // Send updates to service, should work
    // Use Alice and Bob with the same address to check the service can
    // handle multiple internal mappings and that map values can overlap
    let res = update_operation("Alice", ctx, &route_rendezvous).await;
    assert!(res.is_ok(), "Update operation should have been okay");
    let res = update_operation("Bob", ctx, &route_rendezvous).await;
    assert!(res.is_ok(), "Update operation should have been okay");

    // Send queries to service, should work
    let res = query_operation("Alice", ctx, &route_rendezvous).await;
    assert_eq!(res.unwrap(), send_addr, "Unexpected response",);
    let res = query_operation("Bob", ctx, &route_rendezvous).await;
    assert_eq!(res.unwrap(), send_addr, "Unexpected response",);

    // Send query to service, should error
    let res = query_operation("DoesNotExist", ctx, &route_rendezvous).await;
    assert!(res.is_err(), "Query operation should have failed");

    // Shutdown
    ctx.stop().await?;
    Ok(())
}

/// Helper
async fn update_operation(node_name: &str, ctx: &Context, route: &Route) -> Result<()> {
    let msg = RendezvousRequest::Update {
        node_name: String::from(node_name),
    };
    let res: RendezvousResponse = ctx.send_and_receive(route.clone(), msg).await?;
    match res {
        RendezvousResponse::Update(r) => r,
        r => panic!("Invalid response: {:?}", r),
    }
}

/// Helper
async fn query_operation(node_name: &str, ctx: &Context, route: &Route) -> Result<SocketAddr> {
    let msg = RendezvousRequest::Query {
        node_name: String::from(node_name),
    };
    let res: RendezvousResponse = ctx.send_and_receive(route.clone(), msg).await?;
    match res {
        RendezvousResponse::Query(r) => r,
        r => panic!("Invalid response: {:?}", r),
    }
}

/// Echo service that allows us to find out the UDP address the tests are
/// sending from
pub struct EchoUDPAddress;

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
            None => String::from("unknown"),
        };

        // Reply
        debug!("Replying '{}' to {}", src_addr, &msg.return_route());
        ctx.send(msg.return_route(), src_addr).await
    }
}
