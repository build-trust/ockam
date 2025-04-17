use ockam_core::transport::{TransportLocalInfo, TRANSPORT_IDENTIFIER};
use ockam_core::{route, AllowAll, Result};
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpListenerOptions, TcpTransport, TCP};

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn address_metadata__sender__should_be_terminal(ctx: &mut Context) -> Result<()> {
    let transport = TcpTransport::get_or_create(ctx)?;

    let listener = transport
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    ctx.flow_controls()
        .add_consumer(&"test".into(), listener.flow_control_id());

    let connection = transport
        .connect(
            listener.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await?;

    let route = route!["app", connection.clone(), "test"];
    let (address, meta) = ctx.find_terminal_address(route.iter())?.unwrap();

    assert_eq!(address, &connection.clone().into());
    assert_eq!(
        meta.attributes,
        vec![(TRANSPORT_IDENTIFIER.to_string(), TCP.to_string())]
    );
    assert!(meta.is_terminal);

    let mut child_ctx = ctx.new_detached("test", AllowAll, AllowAll)?;

    ctx.send(
        route![connection.clone(), "test"],
        "Hello, Bob!".to_string(),
    )
    .await?;
    let msg = child_ctx.receive::<String>().await?;
    let (address, meta) = ctx
        .find_terminal_address(msg.return_route().iter())?
        .unwrap();

    assert_eq!(
        meta.attributes,
        vec![(TRANSPORT_IDENTIFIER.to_string(), TCP.to_string())]
    );
    assert!(meta.is_terminal);

    let bob_connection = transport
        .registry()
        .get_all_sender_workers()
        .iter()
        .find(|&p| p.address() != connection.sender_address())
        .cloned()
        .unwrap();

    assert_eq!(bob_connection.address(), address);

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn local_info__round_trip__should_be_non_local(ctx: &mut Context) -> Result<()> {
    let transport = TcpTransport::get_or_create(ctx)?;

    let listener = transport
        .listen("127.0.0.1:0", TcpListenerOptions::new())
        .await?;

    ctx.flow_controls()
        .add_consumer(&"test".into(), listener.flow_control_id());

    let connection = transport
        .connect(
            listener.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer(ctx.primary_address(), connection.flow_control_id());

    let mut child_ctx = ctx.new_detached("test", AllowAll, AllowAll)?;

    ctx.send(
        route![connection.clone(), "test"],
        "Hello, Bob!".to_string(),
    )
    .await?;

    let msg = child_ctx.receive::<String>().await?;

    let info = TransportLocalInfo::find_info(msg.local_message())?;
    assert_eq!(info.transport_type(), TCP);

    child_ctx
        .send(msg.return_route().clone(), "test".to_string())
        .await?;

    let msg = ctx.receive::<String>().await?;
    let info = TransportLocalInfo::find_info(msg.local_message())?;
    assert_eq!(info.transport_type(), TCP);

    Ok(())
}
