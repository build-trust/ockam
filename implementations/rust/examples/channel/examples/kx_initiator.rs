use ockam::Address;
use ockam_channel::initiator::XInitiator;
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_router::{
    LocalRouter, Route, Router, RouterAddress, LOCAL_ROUTER_ADDRESS, ROUTER_ADDRESS,
    ROUTER_ADDRESS_TYPE_LOCAL, ROUTER_ADDRESS_TYPE_TCP,
};
use ockam_transport_tcp::{TcpConnection, TcpMessageRouter, TCP_ROUTER_ADDRESS};
use ockam_vault::SoftwareVault;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[ockam::node]
async fn main(ctx: ockam::Context) {
    let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
    let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
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
    let mut connection = TcpConnection::create(SocketAddr::from_str("127.0.0.1:4050").unwrap());
    // let mut connection =
    //     TcpConnection::create(SocketAddr::from_str("138.91.152.195:4000").unwrap());
    if let Err(e) = connection.connect().await {
        ctx.stop().await.unwrap();
        println!("{:?}", e);
        return;
    }
    println!("connected");
    let tcp_router_address = connection.get_router_address();
    let tcp_worker_address = connection.get_worker_address();
    tcp_router.register(tcp_worker_address.clone()).unwrap();

    // create and register the exchange initiator
    let initiator = key_exchanger.initiator();
    let x_initiator = XInitiator {
        m_expected: 0,
        connection_address: (tcp_worker_address.clone()),
        parent: ctx.address(),
        initiator,
        route: Route {
            addrs: vec![
                tcp_router_address,
                RouterAddress {
                    address_type: ROUTER_ADDRESS_TYPE_LOCAL,
                    address: b"x_responder".to_vec(),
                },
            ],
        },
    };
    local_router.register(Address::from("x_initiator")).unwrap();

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
    ctx.start_worker(Address::from("x_initiator"), x_initiator)
        .await
        .unwrap();

    println!("DONE");
}
