use core::time::Duration;

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
    let listener = tcp_bob
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    let tcp_alice = TcpTransport::create(ctx).await?;
    let connection_to_bob = tcp_alice
        .connect(listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = tcp_bob
        .registry()
        .get_all_sender_workers()
        .last()
        .unwrap()
        .clone();

    message_should_not_pass(ctx, &connection_to_bob.clone().into()).await?;
    message_should_not_pass(ctx, connection_to_alice.address()).await?;

    let bob_listener_info =
        create_secure_channel_listener(ctx, listener.flow_control_id(), true).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob.clone().into()).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_not_pass(ctx, &channel_to_bob.address).await?;
    message_should_not_pass(ctx, &channel_to_alice).await?;

    let res = channel_to_bob
        .secure_channels
        .create_secure_channel(
            ctx,
            &channel_to_bob.identifier,
            route![connection_to_bob.clone(), "listener"],
            SecureChannelOptions::new().with_timeout(Duration::from_secs(1)),
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
    let listener = {
        let options = TcpListenerOptions::new();
        tcp_bob.listen("127.0.0.1:0", options).await?
    };

    let tcp_alice = TcpTransport::create(ctx).await?;
    let alice_tcp_options = TcpConnectionOptions::new();
    let alice_flow_control_id = alice_tcp_options.producer_flow_control_id();
    let connection_to_bob = tcp_alice
        .connect(listener.socket_string(), alice_tcp_options)
        .await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = tcp_bob
        .registry()
        .get_all_sender_workers()
        .last()
        .unwrap()
        .clone();

    message_should_not_pass(ctx, &connection_to_bob.into()).await?;
    message_should_not_pass(ctx, connection_to_alice.address()).await?;

    let alice_listener_info =
        create_secure_channel_listener(ctx, &alice_flow_control_id, false).await?;

    let channel_to_alice = create_secure_channel(ctx, connection_to_alice.address()).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_bob = alice_listener_info.get_channel();

    message_should_not_pass(ctx, &channel_to_alice.address).await?;
    message_should_not_pass(ctx, &channel_to_bob).await?;

    ctx.stop().await
}
