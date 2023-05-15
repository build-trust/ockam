use core::time::Duration;

use ockam_core::{route, Result};
use ockam_identity::SecureChannelOptions;
use ockam_node::Context;

use crate::common::{
    create_secure_channel, create_secure_channel_listener, create_tcp_connection_with_flow_control,
    create_tcp_connection_without_flow_control, create_tcp_listener_with_flow_control,
    create_tcp_listener_without_flow_control, message_should_not_pass, message_should_pass,
};

mod common;

// Alice: TCP connection + Secure Channel. No flow_control
// Bob: TCP listener + Secure Channel listener. No flow_control
#[ockam_macros::test]
async fn test1(ctx: &mut Context) -> Result<()> {
    let bob_tcp_info = create_tcp_listener_without_flow_control(ctx).await?;

    let bob_listener_info =
        create_secure_channel_listener(ctx, &bob_tcp_info.flow_control, true).await?;

    let connection_to_bob =
        create_tcp_connection_without_flow_control(ctx, &bob_tcp_info.socket_addr).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = bob_tcp_info.get_connection();

    message_should_pass(ctx, &connection_to_bob.address).await?;
    message_should_pass(ctx, &connection_to_alice.address).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_pass(ctx, &channel_to_bob.address).await?;
    message_should_pass(ctx, &channel_to_alice).await?;

    ctx.stop().await
}

// Alice: TCP connection + Secure Channel. With flow_control
// Bob: TCP listener + Secure Channel listener. No flow_control
#[ockam_macros::test]
async fn test2(ctx: &mut Context) -> Result<()> {
    let bob_tcp_info = create_tcp_listener_without_flow_control(ctx).await?;

    let connection_to_bob =
        create_tcp_connection_with_flow_control(ctx, &bob_tcp_info.socket_addr).await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = bob_tcp_info.get_connection();

    message_should_pass(ctx, &connection_to_bob.address).await?;
    message_should_not_pass(ctx, &connection_to_alice.address).await?;

    let bob_listener_info =
        create_secure_channel_listener(ctx, &bob_tcp_info.flow_control, true).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_pass(ctx, &channel_to_bob.address).await?;
    message_should_pass(ctx, &channel_to_alice).await?;

    let res = channel_to_bob
        .secure_channels
        .create_secure_channel_extended(
            ctx,
            &channel_to_bob.identifier,
            route![connection_to_bob.address.clone(), "listener"],
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

// Alice: TCP connection + Secure Channel. No flow_control
// Bob: TCP listener + Secure Channel listener. With flow_control
#[ockam_macros::test]
async fn test3(ctx: &mut Context) -> Result<()> {
    let bob_tcp_info = create_tcp_listener_with_flow_control(ctx).await?;

    let connection_to_bob =
        create_tcp_connection_without_flow_control(ctx, &bob_tcp_info.socket_addr).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = bob_tcp_info.get_connection();

    // message_should_not_pass(ctx, &connection_to_bob.address).await?;
    message_should_pass(ctx, &connection_to_alice.address).await?;

    let bob_listener_info =
        create_secure_channel_listener(ctx, &bob_tcp_info.flow_control, true).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_pass(ctx, &channel_to_bob.address).await?;
    message_should_pass(ctx, &channel_to_alice).await?;

    let res = channel_to_bob
        .secure_channels
        .create_secure_channel_extended(
            ctx,
            &channel_to_bob.identifier,
            route![connection_to_bob.address.clone(), "listener"],
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

// Alice: TCP connection + Secure Channel. With flow_control
// Bob: TCP listener + Secure Channel listener. With flow_control
#[ockam_macros::test]
async fn test4(ctx: &mut Context) -> Result<()> {
    let bob_tcp_info = create_tcp_listener_with_flow_control(ctx).await?;

    let connection_to_bob =
        create_tcp_connection_with_flow_control(ctx, &bob_tcp_info.socket_addr).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = bob_tcp_info.get_connection();

    message_should_not_pass(ctx, &connection_to_bob.address).await?;
    message_should_not_pass(ctx, &connection_to_alice.address).await?;

    let bob_listener_info =
        create_secure_channel_listener(ctx, &bob_tcp_info.flow_control, true).await?;

    let channel_to_bob = create_secure_channel(ctx, &connection_to_bob).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_listener_info.get_channel();

    message_should_pass(ctx, &channel_to_bob.address).await?;
    message_should_pass(ctx, &channel_to_alice).await?;

    let res = channel_to_bob
        .secure_channels
        .create_secure_channel_extended(
            ctx,
            &channel_to_bob.identifier,
            route![connection_to_bob.address.clone(), "listener"],
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

// Alice: TCP connection + Secure Channel listener. With flow_control
// Bob: TCP listener + Secure Channel. With flow_control
#[ockam_macros::test]
async fn test5(ctx: &mut Context) -> Result<()> {
    let bob_tcp_info = create_tcp_listener_with_flow_control(ctx).await?;

    let connection_to_bob =
        create_tcp_connection_with_flow_control(ctx, &bob_tcp_info.socket_addr).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let connection_to_alice = bob_tcp_info.get_connection();

    message_should_not_pass(ctx, &connection_to_bob.address).await?;
    message_should_not_pass(ctx, &connection_to_alice.address).await?;

    let alice_listener_info =
        create_secure_channel_listener(ctx, &connection_to_bob.flow_control, false).await?;

    let channel_to_alice = create_secure_channel(ctx, &connection_to_alice).await?;
    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_bob = alice_listener_info.get_channel();

    message_should_pass(ctx, &channel_to_alice.address).await?;
    message_should_pass(ctx, &channel_to_bob).await?;

    let res = channel_to_alice
        .secure_channels
        .create_secure_channel_extended(
            ctx,
            &channel_to_alice.identifier,
            route![connection_to_alice.address.clone(), "listener"],
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
