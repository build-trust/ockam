use ockam_core::{route, AllowAll, Mailboxes, Result};
use ockam_identity::{secure_channels, SecureChannelListenerOptions, SecureChannelOptions};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
use std::sync::Arc;

#[ockam_macros::test]
async fn test_update_decryptor_route(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels.create_secure_channel_listener(
        ctx,
        &bob,
        "bob",
        SecureChannelListenerOptions::new(),
    )?;

    let alice_channel = secure_channels
        .create_secure_channel(ctx, &alice, route!["bob"], SecureChannelOptions::new())
        .await?;

    let mut child_ctx = ctx.new_detached_with_mailboxes(Mailboxes::primary(
        "child",
        Arc::new(AllowAll),
        Arc::new(AllowAll),
    ))?;

    ctx.flow_controls()
        .add_consumer(&"child".into(), alice_channel.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"child".into(), bob_listener.flow_control_id());

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.primary_address().clone()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?.into_local_message();

    child_ctx
        .send(msg.return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    assert_eq!("Hello, Alice!", msg.into_body()?);

    alice_channel.update_remote_node_route(route![])?;

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.primary_address().clone()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?.into_local_message();

    child_ctx
        .send(msg.return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    assert_eq!("Hello, Alice!", msg.into_body()?);

    Ok(())
}

#[ockam_macros::test]
async fn test_update_decryptor_route_tcp(ctx: &mut Context) -> Result<()> {
    let tcp = TcpTransport::create(ctx)?;

    let tcp_listener1 = tcp.listen("127.0.0.1:0", TcpListenerOptions::new()).await?;
    let tcp_listener2 = tcp.listen("127.0.0.1:0", TcpListenerOptions::new()).await?;

    let tcp_connection1 = tcp
        .connect(tcp_listener1.socket_string(), TcpConnectionOptions::new())
        .await?;
    let tcp_connection2 = tcp
        .connect(tcp_listener2.socket_string(), TcpConnectionOptions::new())
        .await?;

    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels.create_secure_channel_listener(
        ctx,
        &bob,
        "bob",
        SecureChannelListenerOptions::new()
            .as_consumer(tcp_listener1.flow_control_id())
            .as_consumer(tcp_listener2.flow_control_id()),
    )?;

    let alice_channel = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route![tcp_connection1.clone(), "bob"],
            SecureChannelOptions::new(),
        )
        .await?;

    let mut child_ctx = ctx.new_detached_with_mailboxes(Mailboxes::primary(
        "child",
        Arc::new(AllowAll),
        Arc::new(AllowAll),
    ))?;

    ctx.flow_controls()
        .add_consumer(&"child".into(), alice_channel.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"child".into(), bob_listener.flow_control_id());

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.primary_address().clone()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?.into_local_message();

    child_ctx
        .send(msg.return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    assert_eq!("Hello, Alice!", msg.into_body()?);

    tcp_connection1.stop(ctx)?;

    alice_channel.update_remote_node_route(route![tcp_connection2])?;

    child_ctx
        .send(
            route![alice_channel.clone(), child_ctx.primary_address().clone()],
            "Hello, Bob!".to_string(),
        )
        .await?;

    let msg = child_ctx.receive::<String>().await?.into_local_message();

    child_ctx
        .send(msg.return_route, "Hello, Alice!".to_string())
        .await?;

    let msg = child_ctx.receive::<String>().await?;

    assert_eq!("Hello, Alice!", msg.into_body()?);

    Ok(())
}
