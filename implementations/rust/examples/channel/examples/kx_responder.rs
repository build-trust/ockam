use ockam::Address;
use ockam_channel::channel_factory::{
    ChannelFactoryMessage, XXChannelFactory, XX_CHANNEL_FACTORY_ADDRESS,
};
use ockam_channel::channels_facade::{ChannelsFacade, CHANNELS_FACADE_ADDRESS};
use ockam_router::{
    LocalRouter, Router, LOCAL_ROUTER_ADDRESS, ROUTER_ADDRESS, ROUTER_ADDRESS_TYPE_LOCAL,
    ROUTER_ADDRESS_TYPE_TCP,
};
use ockam_transport_tcp::{TcpListener, TcpMessageRouter, TcpWorkerMessage, TCP_ROUTER_ADDRESS};
use std::net::SocketAddr;
use std::str::FromStr;

struct Workers {
    router: Router,
    local_router: LocalRouter,
    tcp_router: TcpMessageRouter,
    channels_facade: ChannelsFacade,
    xx_channel_factory: XXChannelFactory,
}

async fn create_workers() -> Workers {
    let mut router = Router::new();

    let mut local_router = LocalRouter::new();
    router
        .register(
            ROUTER_ADDRESS_TYPE_LOCAL,
            Address::from(LOCAL_ROUTER_ADDRESS),
        )
        .unwrap();

    let tcp_router = TcpMessageRouter::new();
    router
        .register(ROUTER_ADDRESS_TYPE_TCP, Address::from(TCP_ROUTER_ADDRESS))
        .unwrap();

    let channels_facade = ChannelsFacade::new();
    local_router
        .register(CHANNELS_FACADE_ADDRESS.to_string().into())
        .unwrap();

    let xx_channel_factory = XXChannelFactory::new(Vec::new().into());

    Workers {
        router,
        local_router,
        tcp_router,
        channels_facade,
        xx_channel_factory,
    }
}

async fn start_workers(ctx: &ockam::Context, workers: Workers) {
    ctx.start_worker(ROUTER_ADDRESS, workers.router)
        .await
        .unwrap();

    ctx.start_worker(TCP_ROUTER_ADDRESS, workers.tcp_router)
        .await
        .unwrap();

    ctx.start_worker(LOCAL_ROUTER_ADDRESS, workers.local_router)
        .await
        .unwrap();

    ctx.start_worker(CHANNELS_FACADE_ADDRESS, workers.channels_facade)
        .await
        .unwrap();

    ctx.start_worker(XX_CHANNEL_FACTORY_ADDRESS, workers.xx_channel_factory)
        .await
        .unwrap();
}

#[ockam::node]
async fn main(ctx: ockam::Context) {
    let mut workers = create_workers().await;

    // create and register the tcp connection
    let mut listener = TcpListener::create(SocketAddr::from_str("127.0.0.1:4050").unwrap())
        .await
        .unwrap();
    let tcp_connection = listener.accept().await.unwrap();
    let connection_router_address = tcp_connection.get_router_address();
    let connection_worker_address = tcp_connection.get_worker_address();
    workers
        .tcp_router
        .register(connection_worker_address.clone())
        .unwrap();

    start_workers(&ctx, workers).await;

    // tcp worker
    ctx.start_worker(connection_worker_address.clone(), tcp_connection)
        .await
        .unwrap();

    let create_channel_msg =
        ChannelFactoryMessage::wait_for_initiator(connection_worker_address.clone());

    ctx.send_message(XX_CHANNEL_FACTORY_ADDRESS, create_channel_msg)
        .await
        .unwrap();
}
