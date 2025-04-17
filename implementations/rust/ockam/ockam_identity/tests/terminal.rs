use ockam_abac::{IncomingAbac, OutgoingAbac, PolicyExpression};
use ockam_core::identity::SECURE_CHANNEL_IDENTIFIER;
use ockam_core::{route, AllowAll, Result};
use ockam_identity::{secure_channels, SecureChannelListenerOptions, SecureChannelOptions};
use ockam_node::{Context, MessageReceiveOptions};
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport};
use std::str::FromStr;
use std::time::Duration;

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn address_metadata__encryptor__should_be_terminal(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels.create_secure_channel_listener(
        ctx,
        &bob,
        "bob_listener",
        SecureChannelListenerOptions::new(),
    )?;

    ctx.flow_controls()
        .add_consumer(&"test".into(), bob_listener.flow_control_id());

    let sc = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    let route = route!["app", sc.clone(), "test"];
    let (address, meta) = ctx.find_terminal_address(route.iter())?.unwrap();

    assert_eq!(address, &sc.clone().into());
    assert_eq!(
        meta.attributes,
        vec![(SECURE_CHANNEL_IDENTIFIER.to_string(), hex::encode(bob.0))]
    );
    assert!(meta.is_terminal);

    let mut child_ctx = ctx.new_detached("test", AllowAll, AllowAll)?;

    ctx.send(route![sc.clone(), "test"], "Hello, Bob!".to_string())
        .await?;
    let msg = child_ctx.receive::<String>().await?;
    let (address, meta) = ctx
        .find_terminal_address(msg.return_route().iter())?
        .unwrap();

    assert_eq!(
        meta.attributes,
        vec![(SECURE_CHANNEL_IDENTIFIER.to_string(), hex::encode(alice.0))]
    );
    assert!(meta.is_terminal);

    let bob_sc = secure_channels
        .secure_channel_registry()
        .get_channel_list()
        .iter()
        .find(|&p| p.encryptor_messaging_address() != sc.encryptor_address())
        .cloned()
        .unwrap();

    assert_eq!(bob_sc.encryptor_messaging_address(), address);

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn policy__local__should_evaluate_correctly(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let policy = PolicyExpression::from_str("message.is_local")?;

    let incoming = IncomingAbac::create(
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    );
    let outgoing = OutgoingAbac::create(
        ctx.get_router_context(),
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    )?;

    let mut ctx1 = ctx.new_detached("ctx", incoming, outgoing)?;

    ctx.send(ctx1.primary_address().clone(), "".to_string())
        .await?;
    let msg = ctx1.receive::<String>().await?;

    let return_route = msg.return_route().clone();
    ctx1.send(return_route.clone(), "".to_string()).await?;
    let _msg = ctx.receive::<String>().await?;

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn policy__tcp__should_evaluate_correctly(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let transport = TcpTransport::get_or_create(ctx)?;

    let listener = transport
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    ctx.flow_controls()
        .add_consumer(&"ctx1".into(), listener.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"ctx2".into(), listener.flow_control_id());

    let connection = transport
        .connect(
            listener.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await?;

    let policy = PolicyExpression::from_str("message.is_local")?;

    let incoming = IncomingAbac::create(
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    );
    let outgoing = OutgoingAbac::create(
        ctx.get_router_context(),
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    )?;

    ctx.flow_controls()
        .add_consumer(ctx.primary_address(), connection.flow_control_id());

    let mut ctx1 = ctx.new_detached("ctx1", AllowAll, outgoing)?;
    let mut ctx2 = ctx.new_detached("ctx2", incoming, AllowAll)?;

    ctx.send(
        route![connection.clone(), ctx1.primary_address().clone()],
        "".to_string(),
    )
    .await?;
    let msg = ctx1.receive::<String>().await?;

    let return_route = msg.return_route().clone();
    ctx1.send(return_route.clone(), "".to_string()).await?;
    let msg = ctx
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_secs(1)),
        )
        .await;
    assert!(msg.is_err());

    ctx.send(
        route![connection.clone(), ctx2.primary_address().clone()],
        "".to_string(),
    )
    .await?;
    let msg = ctx2
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_secs(1)),
        )
        .await;
    assert!(msg.is_err());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn policy__secure_channel__should_evaluate_correctly(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let identities_creation = secure_channels.identities().identities_creation();

    let alice = identities_creation.create_identity().await?;
    let bob = identities_creation.create_identity().await?;

    let bob_listener = secure_channels.create_secure_channel_listener(
        ctx,
        &bob,
        "bob_listener",
        SecureChannelListenerOptions::new(),
    )?;

    let sc = secure_channels
        .create_secure_channel(
            ctx,
            &alice,
            route!["bob_listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer(&"ctx1".into(), bob_listener.flow_control_id());
    ctx.flow_controls()
        .add_consumer(&"ctx2".into(), bob_listener.flow_control_id());

    let policy = PolicyExpression::from_str("message.is_local")?;

    let incoming = IncomingAbac::create(
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    );
    let outgoing = OutgoingAbac::create(
        ctx.get_router_context(),
        secure_channels.identities().identities_attributes(),
        None,
        policy.to_expression(),
    )?;

    ctx.flow_controls()
        .add_consumer(ctx.primary_address(), sc.flow_control_id());

    let mut ctx1 = ctx.new_detached("ctx1", AllowAll, outgoing)?;
    let mut ctx2 = ctx.new_detached("ctx2", incoming, AllowAll)?;

    ctx.send(
        route![sc.clone(), ctx1.primary_address().clone()],
        "".to_string(),
    )
    .await?;
    let msg = ctx1.receive::<String>().await?;

    let return_route = msg.return_route().clone();
    ctx1.send(return_route.clone(), "".to_string()).await?;
    let msg = ctx
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_secs(1)),
        )
        .await;
    assert!(msg.is_err());

    ctx.send(
        route![sc.clone(), ctx2.primary_address().clone()],
        "".to_string(),
    )
    .await?;
    let msg = ctx2
        .receive_extended::<String>(
            MessageReceiveOptions::new().with_timeout(Duration::from_secs(1)),
        )
        .await;
    assert!(msg.is_err());

    Ok(())
}
