use ockam::Address;
use ockam_channel::responder::XResponder;
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_router::{
    LocalRouter, Router, LOCAL_ROUTER_ADDRESS, ROUTER_ADDRESS, ROUTER_ADDRESS_TYPE_LOCAL,
    ROUTER_ADDRESS_TYPE_TCP,
};
use ockam_transport_tcp::{TcpMessageRouter, TcpWorkerMessage, TCP_ROUTER_ADDRESS};
use ockam_vault::SoftwareVault;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[ockam::node]
async fn main(ctx: ockam::Context) {
    let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
    let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
    let key_exchanger = XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());

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

    // create and register the exchange responder
    let responder = key_exchanger.responder();
    let x_responder = XResponder {
        m_expected: 0,
        connection_address: (tcp_worker_address.clone()),
        parent: (ctx.address()),
        responder: (responder),
    };
    local_router.register(Address::from("x_responder")).unwrap();

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

    // start the worker
    ctx.start_worker(Address::from("x_responder"), x_responder)
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
