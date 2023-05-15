use ockam::remote::{RemoteForwarder, RemoteForwarderOptions};
use ockam::workers::Echoer;
use ockam::ForwardingService;
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_core::{route, Address, AllowAll, Result};
use ockam_identity::{secure_channels, SecureChannelListenerOptions, SecureChannelOptions};
use ockam_node::{Context, MessageReceiveOptions, MessageSendReceiveOptions};
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
use std::time::Duration;

// Node creates a Forwarding service and a Remote Forwarder, Echoer is reached through the Forwarder. No flow control
#[ockam_macros::test]
async fn test1(ctx: &mut Context) -> Result<()> {
    ForwardingService::create(ctx, "forwarding_service", AllowAll, AllowAll).await?;

    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
        .await?;

    let remote_info = RemoteForwarder::create(ctx, route![], RemoteForwarderOptions::new()).await?;

    let resp = ctx
        .send_and_receive::<String>(
            route![remote_info.remote_address(), "echoer"],
            "Hello".to_string(),
        )
        .await?;

    assert_eq!(resp, "Hello");

    ctx.stop().await
}

// Cloud: Hosts a Forwarding service and listens on a tcp port. No flow control
// Server: Connects to a Cloud using tcp and creates a dynamic Forwarder. Using flow control
// Client: Connects to a Cloud using tcp and reaches to the Server's Echoer. Using flow control
#[ockam_macros::test]
async fn test2(ctx: &mut Context) -> Result<()> {
    ForwardingService::create(ctx, "forwarding_service", AllowAll, AllowAll).await?;
    let cloud_tcp = TcpTransport::create(ctx).await?;
    let (socket_addr, _) = cloud_tcp
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    let server_flow_controls = FlowControls::default();
    let server_tcp_flow_control_id = server_flow_controls.generate_id();

    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
        .await?;
    server_flow_controls.add_consumer(
        &Address::from_string("echoer"),
        &server_tcp_flow_control_id,
        FlowControlPolicy::ProducerAllowMultiple,
    );

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = server_tcp
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&server_flow_controls, &server_tcp_flow_control_id),
        )
        .await?;

    let remote_info = RemoteForwarder::create(
        ctx,
        cloud_connection.clone(),
        RemoteForwarderOptions::as_consumer_and_producer(&server_flow_controls),
    )
    .await?;

    let client_flow_controls = FlowControls::default();
    let client_tcp_flow_control_id = client_flow_controls.generate_id();

    let client_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = client_tcp
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&client_flow_controls, &client_tcp_flow_control_id),
        )
        .await?;

    let resp = ctx
        .send_and_receive_extended::<String>(
            route![cloud_connection, remote_info.remote_address(), "echoer"],
            "Hello".to_string(),
            MessageSendReceiveOptions::new().with_flow_control(&client_flow_controls),
        )
        .await?
        .body();

    assert_eq!(resp, "Hello");

    ctx.stop().await
}

// Server: Connects to a Cloud using tcp and creates a dynamic Forwarder. Using flow control
// Cloud: Hosts a Forwarding service and sends replies to the Client with and without a flow control
#[ockam_macros::test]
async fn test3(ctx: &mut Context) -> Result<()> {
    ForwardingService::create(ctx, "forwarding_service", AllowAll, AllowAll).await?;
    let cloud_tcp = TcpTransport::create(ctx).await?;
    let (socket_addr, _) = cloud_tcp
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    let server_flow_controls = FlowControls::default();
    let server_tcp_flow_control_id = server_flow_controls.generate_id();

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = server_tcp
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&server_flow_controls, &server_tcp_flow_control_id),
        )
        .await?;

    let remote_info = RemoteForwarder::create(
        ctx,
        cloud_connection.clone(),
        RemoteForwarderOptions::as_consumer_and_producer(&server_flow_controls),
    )
    .await?;

    let mut child_ctx = ctx.new_detached("ctx", AllowAll, AllowAll).await?;
    ctx.send(
        route![remote_info.remote_address(), "ctx"],
        "Hello".to_string(),
    )
    .await?;

    let res = child_ctx
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_millis(100)),
        )
        .await;

    assert!(res.is_err(), "Should not pass outgoing access control");

    server_flow_controls.add_consumer(
        &Address::from_string("ctx"),
        &server_tcp_flow_control_id,
        FlowControlPolicy::ProducerAllowMultiple,
    );

    ctx.send(
        route![remote_info.remote_address(), "ctx"],
        "Hello".to_string(),
    )
    .await?;

    let res = child_ctx
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_millis(100)),
        )
        .await?;

    assert_eq!(res.body(), "Hello");

    ctx.stop().await
}

// Cloud:
//  - Hosts a Forwarding service
//  - Listens on a tcp port without a flow control
//  - Runs a secure channel listener
//
// Server:
//  - Connects to the Cloud using tcp with a flow control
//  - Creates a secure channel to the Cloud with a flow control
//  - Creates a dynamic Forwarder. Using flow control
//  - Runs a Secure Channel listener with a flow control
//  - Runs an Echoer
//
// Client:
//  - Connects to a Cloud using tcp with a flow control
//  - Creates a secure channel to the Cloud with a flow control
//  - Creates a tunneled secure channel to the server using Forwarder's address
//  - Reaches Server's Echoer using a flow control
#[ockam_macros::test]
async fn test4(ctx: &mut Context) -> Result<()> {
    // Cloud
    ForwardingService::create(ctx, "forwarding_service", AllowAll, AllowAll).await?;

    let secure_channels = secure_channels();
    let identities_creation = secure_channels.identities().identities_creation();
    let cloud_identity = identities_creation.create_identity().await?;
    secure_channels
        .create_secure_channel_listener(
            ctx,
            &cloud_identity.identifier(),
            "cloud_listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let cloud_tcp = TcpTransport::create(ctx).await?;
    let (socket_addr, _) = cloud_tcp
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    // Server
    let server_flow_controls = FlowControls::default();
    let server_tcp_flow_control_id = server_flow_controls.generate_id();
    let server_channel_flow_control_id = server_flow_controls.generate_id();
    let server_tunnel_flow_control_id = server_flow_controls.generate_id();

    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll)
        .await?;
    server_flow_controls.add_consumer(
        &Address::from_string("echoer"),
        &server_tunnel_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_server_connection = server_tcp
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&server_flow_controls, &server_tcp_flow_control_id),
        )
        .await?;
    let server_identity = identities_creation.create_identity().await?;
    let cloud_server_channel = secure_channels
        .create_secure_channel(
            ctx,
            &server_identity.identifier(),
            route![cloud_server_connection, "cloud_listener"],
            SecureChannelOptions::as_producer(
                &server_flow_controls,
                &server_channel_flow_control_id,
            )
            .as_consumer(&server_flow_controls),
        )
        .await?;
    secure_channels
        .create_secure_channel_listener(
            ctx,
            &server_identity.identifier(),
            "server_listener",
            SecureChannelListenerOptions::as_spawner(
                &server_flow_controls,
                &server_tunnel_flow_control_id,
            )
            .as_consumer_with_flow_control_id(
                &server_flow_controls,
                &server_channel_flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            ),
        )
        .await?;

    let remote_info = RemoteForwarder::create(
        ctx,
        cloud_server_channel.clone(),
        RemoteForwarderOptions::as_consumer_and_producer(&server_flow_controls),
    )
    .await?;

    // Client
    let client_flow_controls = FlowControls::default();
    let client_tcp_flow_control_id = client_flow_controls.generate_id();
    let client_channel_flow_control_id = client_flow_controls.generate_id();
    let client_tunnel_flow_control_id = client_flow_controls.generate_id();

    let client_tcp = TcpTransport::create(ctx).await?;
    let cloud_client_connection = client_tcp
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&client_flow_controls, &client_tcp_flow_control_id),
        )
        .await?;
    let client_identity = identities_creation.create_identity().await?;
    let cloud_client_channel = secure_channels
        .create_secure_channel(
            ctx,
            &client_identity.identifier(),
            route![cloud_client_connection, "cloud_listener"],
            SecureChannelOptions::as_producer(
                &client_flow_controls,
                &client_channel_flow_control_id,
            )
            .as_consumer(&client_flow_controls),
        )
        .await?;

    let tunnel_channel = secure_channels
        .create_secure_channel(
            ctx,
            &client_identity.identifier(),
            route![
                cloud_client_channel,
                remote_info.remote_address(),
                "server_listener"
            ],
            SecureChannelOptions::as_producer(
                &client_flow_controls,
                &client_tunnel_flow_control_id,
            )
            .as_consumer(&client_flow_controls),
        )
        .await?;

    let resp = ctx
        .send_and_receive_extended::<String>(
            route![tunnel_channel, "echoer"],
            "Hello".to_string(),
            MessageSendReceiveOptions::new().with_flow_control(&client_flow_controls),
        )
        .await?
        .body();

    assert_eq!(resp, "Hello");

    ctx.stop().await
}
