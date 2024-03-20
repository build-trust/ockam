use ockam::identity::{secure_channels, SecureChannelListenerOptions, SecureChannelOptions};
use ockam::remote::{RemoteRelay, RemoteRelayOptions};
use ockam::workers::Echoer;
use ockam::{RelayService, RelayServiceOptions};
use ockam_core::{route, AllowAll, Result};
use ockam_node::{Context, MessageReceiveOptions};
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
use std::time::Duration;

// Node creates a Relay service and a Remote Relay, Echoer is reached through the Relay. No flow control
#[ockam_macros::test]
async fn test1(ctx: &mut Context) -> Result<()> {
    RelayService::create(ctx, "forwarding_service", RelayServiceOptions::new()).await?;

    ctx.start_worker("echoer", Echoer).await?;

    let remote_info = RemoteRelay::create(ctx, route![], RemoteRelayOptions::new()).await?;

    let resp = ctx
        .send_and_receive::<String>(
            route![remote_info.remote_address(), "echoer"],
            "Hello".to_string(),
        )
        .await?;

    assert_eq!(resp, "Hello");
    Ok(())
}

// Cloud: Hosts a Relay service and listens on a tcp port. No flow control
// Server: Connects to a Cloud using tcp and creates a dynamic Relay. Using flow control
// Client: Connects to a Cloud using tcp and reaches to the Server's Echoer. Using flow control
#[ockam_macros::test]
async fn test2(ctx: &mut Context) -> Result<()> {
    let tcp_listener_options = TcpListenerOptions::new();
    let options = RelayServiceOptions::new()
        .service_as_consumer(&tcp_listener_options.spawner_flow_control_id())
        .relay_as_consumer(&tcp_listener_options.spawner_flow_control_id());
    RelayService::create(ctx, "forwarding_service", options).await?;
    let cloud_tcp = TcpTransport::create(ctx).await?;

    let cloud_listener = cloud_tcp
        .listen("127.0.0.1:0", tcp_listener_options)
        .await?;

    let tcp_options = TcpConnectionOptions::new();

    ctx.start_worker("echoer", Echoer).await?;
    ctx.flow_controls()
        .add_consumer("echoer", &tcp_options.flow_control_id());

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = server_tcp
        .connect(cloud_listener.socket_string(), tcp_options)
        .await?;

    let remote_info =
        RemoteRelay::create(ctx, cloud_connection.clone(), RemoteRelayOptions::new()).await?;

    let client_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = client_tcp
        .connect(cloud_listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    let resp = ctx
        .send_and_receive::<String>(
            route![cloud_connection, remote_info.remote_address(), "echoer"],
            "Hello".to_string(),
        )
        .await?;

    assert_eq!(resp, "Hello");
    Ok(())
}

// Server: Connects to a Cloud using tcp and creates a dynamic Relay. Using flow control
// Cloud: Hosts a Relay service and sends replies to the Client with and without a flow control
#[ockam_macros::test]
async fn test3(ctx: &mut Context) -> Result<()> {
    let tcp_listener_options = TcpListenerOptions::new();
    let options = RelayServiceOptions::new()
        .service_as_consumer(&tcp_listener_options.spawner_flow_control_id())
        .relay_as_consumer(&tcp_listener_options.spawner_flow_control_id());
    RelayService::create(ctx, "forwarding_service", options).await?;
    let cloud_tcp = TcpTransport::create(ctx).await?;
    let cloud_listener = cloud_tcp
        .listen("127.0.0.1:0", tcp_listener_options)
        .await?;

    let tcp_options = TcpConnectionOptions::new();
    let server_tcp_flow_control_id = tcp_options.flow_control_id();

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_connection = server_tcp
        .connect(cloud_listener.socket_string(), tcp_options)
        .await?;

    let remote_info =
        RemoteRelay::create(ctx, cloud_connection.clone(), RemoteRelayOptions::new()).await?;

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

    ctx.flow_controls()
        .add_consumer("ctx", &server_tcp_flow_control_id);

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

    assert_eq!(res.into_body()?, "Hello");
    Ok(())
}

// Cloud:
//  - Hosts a Relay service
//  - Listens on a tcp port without a flow control
//  - Runs a secure channel listener
//
// Server:
//  - Connects to the Cloud using tcp with a flow control
//  - Creates a secure channel to the Cloud with a flow control
//  - Creates a dynamic Relay. Using flow control
//  - Runs a Secure Channel listener with a flow control
//  - Runs an Echoer
//
// Client:
//  - Connects to a Cloud using tcp with a flow control
//  - Creates a secure channel to the Cloud with a flow control
//  - Creates a tunneled secure channel to the server using Relay's address
//  - Reaches Server's Echoer using a flow control
#[ockam_macros::test]
async fn test4(ctx: &mut Context) -> Result<()> {
    // Cloud
    let cloud_tcp_listener_options = TcpListenerOptions::new();
    let cloud_secure_channel_listener_options = SecureChannelListenerOptions::new()
        .as_consumer(&cloud_tcp_listener_options.spawner_flow_control_id());

    let options = RelayServiceOptions::new()
        .service_as_consumer(&cloud_secure_channel_listener_options.spawner_flow_control_id())
        .relay_as_consumer(&cloud_secure_channel_listener_options.spawner_flow_control_id());
    RelayService::create(ctx, "forwarding_service", options).await?;

    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();
    let cloud = identities_creation.create_identity().await?;
    secure_channels
        .create_secure_channel_listener(
            ctx,
            &cloud,
            "cloud_listener",
            cloud_secure_channel_listener_options,
        )
        .await?;

    let cloud_tcp = TcpTransport::create(ctx).await?;
    let cloud_listener = cloud_tcp
        .listen("127.0.0.1:0", cloud_tcp_listener_options)
        .await?;

    // Server
    let server_secure_channel_options = SecureChannelOptions::new();
    let server_secure_channel_listener_options = SecureChannelListenerOptions::new()
        .as_consumer(&server_secure_channel_options.producer_flow_control_id());

    ctx.start_worker("echoer", Echoer).await?;
    ctx.flow_controls().add_consumer(
        "echoer",
        &server_secure_channel_listener_options.spawner_flow_control_id(),
    );

    let server_tcp = TcpTransport::create(ctx).await?;
    let cloud_server_connection = server_tcp
        .connect(cloud_listener.socket_string(), TcpConnectionOptions::new())
        .await?;
    let server = identities_creation.create_identity().await?;
    let cloud_server_channel = secure_channels
        .create_secure_channel(
            ctx,
            &server,
            route![cloud_server_connection, "cloud_listener"],
            server_secure_channel_options,
        )
        .await?;
    secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "server_listener",
            server_secure_channel_listener_options,
        )
        .await?;

    let remote_info =
        RemoteRelay::create(ctx, cloud_server_channel.clone(), RemoteRelayOptions::new()).await?;

    // Client
    let client_tcp = TcpTransport::create(ctx).await?;
    let cloud_client_connection = client_tcp
        .connect(cloud_listener.socket_string(), TcpConnectionOptions::new())
        .await?;
    let client = identities_creation.create_identity().await?;
    let cloud_client_channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route![cloud_client_connection, "cloud_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let tunnel_channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route![
                cloud_client_channel,
                remote_info.remote_address(),
                "server_listener"
            ],
            SecureChannelOptions::new(),
        )
        .await?;

    let resp = ctx
        .send_and_receive::<String>(route![tunnel_channel, "echoer"], "Hello".to_string())
        .await?;

    assert_eq!(resp, "Hello");

    Ok(())
}
