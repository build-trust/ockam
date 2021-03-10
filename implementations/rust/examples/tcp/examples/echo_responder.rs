use async_trait::async_trait;
use ockam::{Address, Context, Result, Routed, Worker};
use ockam_router::{
    LocalRouter, RouteTransportMessage, RouteableAddress, Router, TransportMessage,
    LOCAL_ROUTER_ADDRESS, ROUTER_ADDRESS, ROUTER_ADDRESS_TYPE_LOCAL, ROUTER_ADDRESS_TYPE_TCP,
};
use ockam_transport_tcp::{TcpMessageRouter, TcpWorkerMessage, TCP_ROUTER_ADDRESS};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::time::{sleep, Duration};

pub struct ResponderEchoRelay {}

impl ResponderEchoRelay {
    pub fn new() -> Self {
        ResponderEchoRelay {}
    }
}

#[async_trait]
impl Worker for ResponderEchoRelay {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        return match msg.take() {
            RouteTransportMessage::Route(m) => {
                println!(
                    "echoing \"{}\"",
                    String::from_utf8(m.payload.clone()).unwrap()
                );
                let mut reply = TransportMessage::new();
                reply.onward_route = m.return_route.clone();
                reply.return_address(RouteableAddress::Local(ctx.address().to_vec()));
                reply.payload = m.payload.clone();
                ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(reply))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(500)).await;
                ctx.stop().await.unwrap();
                Ok(())
            }
            _ => Ok(()),
        };
    }
}

#[ockam::node]
async fn main(ctx: ockam::Context) {
    // create and register everything
    // main router
    let mut router = Router::new();

    // local router
    let mut local_router = LocalRouter::new();
    if let Err(e) = router.register(
        ROUTER_ADDRESS_TYPE_LOCAL,
        Address::from(LOCAL_ROUTER_ADDRESS),
    ) {
        println!("{:?}", e);
        ctx.stop().await.unwrap();
    }

    // tcp router
    let mut tcp_router = TcpMessageRouter::new();
    if let Err(e) = router.register(ROUTER_ADDRESS_TYPE_TCP, Address::from(TCP_ROUTER_ADDRESS)) {
        println!("{:?}", e);
        ctx.stop().await.unwrap();
    }

    // create and register the tcp connection
    let listen_addr = SocketAddr::from_str("127.0.0.1:4050").unwrap();
    let mut listener = ockam_transport_tcp::TcpListener::create(listen_addr)
        .await
        .unwrap();
    let connection = listener.accept().await.unwrap();
    let tcp_worker_address = connection.get_worker_address();
    tcp_router.register(tcp_worker_address.clone()).unwrap();

    // create and register the echo message relay
    let relay = ResponderEchoRelay::new();
    let echo_service_addr = Address::from("echo_service");
    local_router
        .register(Address::from("echo_service"))
        .unwrap();

    // start all the workers
    // start the main router
    ctx.start_worker(ROUTER_ADDRESS, router).await.unwrap();

    // start the tcp router
    ctx.start_worker(TCP_ROUTER_ADDRESS, tcp_router)
        .await
        .unwrap();

    // tcp worker
    ctx.start_worker(tcp_worker_address.clone(), connection)
        .await
        .unwrap();

    // start the local router
    ctx.start_worker(LOCAL_ROUTER_ADDRESS, local_router)
        .await
        .unwrap();

    // start the relay worker
    ctx.start_worker(echo_service_addr.clone(), relay)
        .await
        .unwrap();

    // wait for the message
    if ctx
        .send_message(tcp_worker_address, TcpWorkerMessage::Receive)
        .await
        .is_err()
    {
        println!("error receiving message");
        ctx.stop().await.unwrap();
    }
}
