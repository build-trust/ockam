use core::time::Duration;

use ockam_core::flow_control::FlowControls;
use ockam_core::{route, Result};
use ockam_identity::SecureChannelOptions;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};

use crate::common::{
    create_secure_channel, create_secure_channel_listener, message_should_not_pass,
};

mod common;

// Alice: TCP connection + Secure Channel
// Bob: TCP listener + Secure Channel listener
#[ockam_macros::test]
async fn test1(ctx: &mut Context) -> Result<()> {
    let tcp_bob = TcpTransport::create(ctx).await?;
    let (socket_addr, bob_flow_control_id) = {
        let flow_control_id = FlowControls::generate_id();
        let options = TcpListenerOptions::new(&flow_control_id);
        let (socket_addr, _) = tcp_bob.listen("127.0.0.1:0", options).await?;
        (socket_addr, flow_control_id)
    };

    let tcp_alice = TcpTransport::create(ctx).await?;
    let connection_to_bob = tcp_alice
        .connect(socket_addr.to_string(), TcpConnectionOptions::new())
        .await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = tcp_bob
        .registry()
        .get_all_sender_workers()
        .last()
        .unwrap()
        .clone();

    message_should_not_pass(ctx, &connection_to_bob).await?;
    message_should_not_pass(ctx, &connection_to_alice).await?;

    let bob_listener_info = create_secure_channel_listener(ctx, &bob_flow_control_id, true).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_not_pass(ctx, &channel_to_bob.address).await?;
    message_should_not_pass(ctx, &channel_to_alice).await?;

    let res = channel_to_bob
        .secure_channels
        .create_secure_channel_extended(
            ctx,
            &channel_to_bob.identifier,
            route![connection_to_bob.clone(), "listener"],
            SecureChannelOptions::new(),
            Duration::from_secs(1),
        )
        .await;
    assert!(
        res.is_err(),
        "We can only create 1 secure channel with that connection"
    );

    ctx.stop().await
}

// Alice: TCP connection + Secure Channel listener
// Bob: TCP listener + Secure Channel
#[ockam_macros::test]
async fn test2(ctx: &mut Context) -> Result<()> {
    let tcp_bob = TcpTransport::create(ctx).await?;
    let socket_addr = {
        let flow_control_id = FlowControls::generate_id();
        let options = TcpListenerOptions::new(&flow_control_id);
        let (socket_addr, _) = tcp_bob.listen("127.0.0.1:0", options).await?;
        socket_addr
    };

    let tcp_alice = TcpTransport::create(ctx).await?;
    let alice_flow_control_id = FlowControls::generate_id();
    let connection_to_bob = tcp_alice
        .connect(
            socket_addr.to_string(),
            TcpConnectionOptions::as_producer(&alice_flow_control_id),
        )
        .await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = tcp_bob
        .registry()
        .get_all_sender_workers()
        .last()
        .unwrap()
        .clone();

    message_should_not_pass(ctx, &connection_to_bob).await?;
    message_should_not_pass(ctx, &connection_to_alice).await?;

    let alice_listener_info =
        create_secure_channel_listener(ctx, &alice_flow_control_id, false).await?;

    let channel_to_alice = create_secure_channel(ctx, &connection_to_alice).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_bob = alice_listener_info.get_channel();

    message_should_not_pass(ctx, &channel_to_alice.address).await?;
    message_should_not_pass(ctx, &channel_to_bob).await?;

    ctx.stop().await
}
