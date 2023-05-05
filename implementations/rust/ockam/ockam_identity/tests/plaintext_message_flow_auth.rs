use crate::common::{
    message_should_not_pass, message_should_not_pass_with_ctx, message_should_pass_with_ctx,
};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{route, AllowAll, Result};
use ockam_identity::{secure_channels, SecureChannelListenerOptions, SecureChannelOptions};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
use std::time::Duration;

mod common;

// Alice: Secure Channel
// Bob: Secure Channel listener
#[ockam_macros::test]
async fn test1(ctx: &mut Context) -> Result<()> {
    let alice_secure_channels = secure_channels();
    let bob_secure_channels = secure_channels();

    let alice = alice_secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let bob = bob_secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    let bob_listener = bob_secure_channels
        .create_secure_channel_listener(
            ctx,
            &bob.identifier(),
            "listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let channel_to_bob = alice_secure_channels
        .create_secure_channel(
            ctx,
            &alice.identifier(),
            route!["listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let channel_to_alice = bob_secure_channels
        .secure_channel_registry()
        .get_channel_list()
        .first()
        .unwrap()
        .encryptor_messaging_address()
        .clone();

    let mut bob_ctx = ctx.new_detached("bob_ctx", AllowAll, AllowAll).await?;
    message_should_not_pass_with_ctx(ctx, channel_to_bob.encryptor_address(), &mut bob_ctx).await?;
    ctx.flow_controls().add_consumer(
        "bob_ctx",
        bob_listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    message_should_pass_with_ctx(ctx, channel_to_bob.encryptor_address(), &mut bob_ctx).await?;

    let mut alice_ctx = ctx.new_detached("alice_ctx", AllowAll, AllowAll).await?;
    message_should_not_pass_with_ctx(ctx, &channel_to_alice, &mut alice_ctx).await?;
    ctx.flow_controls().add_consumer(
        "alice_ctx",
        channel_to_bob.flow_control_id(),
        FlowControlPolicy::ProducerAllowMultiple,
    );
    message_should_pass_with_ctx(ctx, &channel_to_alice, &mut alice_ctx).await?;

    ctx.stop().await
}

// Alice: TCP connection + Secure Channel
// Bob: TCP listener + Secure Channel listener
#[ockam_macros::test]
async fn test2(ctx: &mut Context) -> Result<()> {
    let tcp_alice = TcpTransport::create(ctx).await?;
    let tcp_bob = TcpTransport::create(ctx).await?;

    let listener = tcp_bob
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    let connection_to_bob = tcp_alice
        .connect(listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry
    let senders = tcp_bob.registry().get_all_sender_workers();
    assert_eq!(senders.len(), 1);

    let connection_to_alice = senders.first().unwrap().clone();

    message_should_not_pass(ctx, &connection_to_bob.clone().into()).await?;
    message_should_not_pass(ctx, connection_to_alice.address()).await?;

    let alice_secure_channels = secure_channels();
    let bob_secure_channels = secure_channels();

    let alice = alice_secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let bob = bob_secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    let bob_options = SecureChannelListenerOptions::new().as_consumer(
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowOnlyOneMessage,
    );
    let bob_listener = bob_secure_channels
        .create_secure_channel_listener(ctx, &bob.identifier(), "listener", bob_options)
        .await?;

    let channel_to_bob = alice_secure_channels
        .create_secure_channel(
            ctx,
            &alice.identifier(),
            route![connection_to_bob, "listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    ctx.sleep(Duration::from_millis(50)).await; // Wait for workers to add themselves to the registry

    let channels = bob_secure_channels
        .secure_channel_registry()
        .get_channel_list();
    assert_eq!(channels.len(), 1);
    let channel_to_alice = channels
        .first()
        .unwrap()
        .encryptor_messaging_address()
        .clone();

    let mut bob_ctx = ctx.new_detached("bob_ctx", AllowAll, AllowAll).await?;
    message_should_not_pass_with_ctx(ctx, channel_to_bob.encryptor_address(), &mut bob_ctx).await?;
    ctx.flow_controls().add_consumer(
        "bob_ctx",
        bob_listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    message_should_pass_with_ctx(ctx, channel_to_bob.encryptor_address(), &mut bob_ctx).await?;

    let mut alice_ctx = ctx.new_detached("alice_ctx", AllowAll, AllowAll).await?;
    message_should_not_pass_with_ctx(ctx, &channel_to_alice, &mut alice_ctx).await?;
    ctx.flow_controls().add_consumer(
        "alice_ctx",
        channel_to_bob.flow_control_id(),
        FlowControlPolicy::ProducerAllowMultiple,
    );
    message_should_pass_with_ctx(ctx, &channel_to_alice, &mut alice_ctx).await?;

    ctx.stop().await
}
