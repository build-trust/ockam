use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use ockam_core::compat::rand::random;
use ockam_core::{route, Result};
use ockam_node::Context;
use ockam_transport_tcp::{
    TcpConnectionOptions, TcpInletOptions, TcpListenerOptions, TcpOutletOptions, TcpTransport,
};

const LENGTH: usize = 32;

async fn setup(ctx: &Context) -> Result<(String, TcpListener)> {
    let tcp = TcpTransport::create(ctx).await?;

    let listener = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_address = listener.local_addr().unwrap().to_string();
        tcp.create_outlet(
            "outlet",
            bind_address.try_into().unwrap(),
            TcpOutletOptions::new(),
        )
        .await?;
        listener
    };

    let inlet = tcp
        .create_inlet("127.0.0.1:0", route!["outlet"], TcpInletOptions::new())
        .await?;

    Ok((inlet.socket_address().to_string(), listener))
}

fn generate_binary() -> [u8; LENGTH] {
    random()
}

async fn write_binary(stream: &mut TcpStream, payload: [u8; LENGTH]) {
    stream.write_all(&payload).await.unwrap();
}

async fn read_assert_binary(stream: &mut TcpStream, expected_payload: [u8; LENGTH]) {
    let mut payload = [0u8; LENGTH];
    let length = stream.read_exact(&mut payload).await.unwrap();
    assert_eq!(length, LENGTH);
    assert_eq!(payload, expected_payload);
}

async fn read_should_timeout(stream: &mut TcpStream) {
    let mut payload = [0u8; LENGTH];
    let res = stream.try_read(&mut payload);
    assert!(res.is_err(), "Read should timeout");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let res = stream.try_read(&mut payload);
    assert!(res.is_err(), "Read should timeout");
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn portal__standard_flow__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let (inlet_addr, listener) = setup(ctx).await?;

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        read_assert_binary(&mut stream, payload1).await;
        write_binary(&mut stream, payload2).await;
        stream
    });

    // Wait till the listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet_addr).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    let res = handle.await;
    assert!(res.is_ok());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn portal__reverse_flow__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let (inlet_addr, listener) = setup(ctx).await?;

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        write_binary(&mut stream, payload2).await;
        read_assert_binary(&mut stream, payload1).await;
        stream
    });

    // Wait till listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet_addr).await.unwrap();
    read_assert_binary(&mut stream, payload2).await;
    write_binary(&mut stream, payload1).await;

    let res = handle.await;
    assert!(res.is_ok());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 15000)]
async fn portal__tcp_connection__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let options = TcpListenerOptions::new();
    let outlet_flow_control_id = options.spawner_flow_control_id();

    let tcp = TcpTransport::create(ctx).await?;

    let listener = tcp.listen("127.0.0.1:0", options).await?;

    let tcp_connection = tcp
        .connect(
            listener.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await?;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bind_address = listener.local_addr().unwrap().to_string();
    tcp.create_outlet(
        "outlet",
        bind_address.try_into().unwrap(),
        TcpOutletOptions::new().as_consumer(&outlet_flow_control_id),
    )
    .await?;

    let inlet = tcp
        .create_inlet(
            "127.0.0.1:0",
            route![tcp_connection.clone(), "outlet"],
            TcpInletOptions::new(),
        )
        .await?;

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        write_binary(&mut stream, payload2).await;
        read_assert_binary(&mut stream, payload1).await;
    });

    // Wait till listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet.socket_address()).await.unwrap();
    read_assert_binary(&mut stream, payload2).await;
    write_binary(&mut stream, payload1).await;

    let res = handle.await;
    assert!(res.is_ok());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 15000)]
async fn portal__tcp_connection_with_invalid_message_flow__should_not_succeed(
    ctx: &mut Context,
) -> Result<()> {
    let payload = generate_binary();

    let options = TcpListenerOptions::new();

    let tcp = TcpTransport::create(ctx).await?;

    let tcp_listener = tcp.listen("127.0.0.1:0", options).await?;

    let tcp_connection = tcp
        .connect(tcp_listener.socket_string(), TcpConnectionOptions::new())
        .await?;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bind_address = listener.local_addr().unwrap().to_string();

    tcp.create_outlet(
        "outlet_invalid",
        bind_address.try_into().unwrap(),
        TcpOutletOptions::new(),
    )
    .await?;

    let inlet = tcp
        .create_inlet(
            "127.0.0.1:0",
            route![tcp_connection, "outlet_invalid"],
            TcpInletOptions::new(),
        )
        .await?;

    let handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                write_binary(&mut stream, payload).await;
            });
        }
    });

    // Wait till listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet.socket_address()).await.unwrap();
    read_should_timeout(&mut stream).await;

    handle.abort();

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn portal__update_route__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let tcp = TcpTransport::create(ctx).await?;

    let listener_outlet = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener_node = tcp.listen("127.0.0.1:0", TcpListenerOptions::new()).await?;

    tcp.create_outlet(
        "outlet",
        listener_outlet
            .local_addr()
            .unwrap()
            .to_string()
            .try_into()?,
        TcpOutletOptions::new().as_consumer(listener_node.flow_control_id()),
    )
    .await?;

    let node_connection1 = tcp
        .connect(
            listener_node.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await?;
    let node_connection2 = tcp
        .connect(
            listener_node.socket_address().to_string(),
            TcpConnectionOptions::new(),
        )
        .await
        .unwrap();

    let inlet = tcp
        .create_inlet(
            "127.0.0.1:0",
            route![node_connection1.clone(), "outlet"],
            TcpInletOptions::new(),
        )
        .await?;

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener_outlet.accept().await.unwrap();

        read_assert_binary(&mut stream, payload1).await;
        write_binary(&mut stream, payload2).await;

        let (mut stream, _) = listener_outlet.accept().await.unwrap();

        read_assert_binary(&mut stream, payload1).await;
        write_binary(&mut stream, payload2).await;

        stream
    });

    // Wait till the listener is up
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(inlet.socket_address()).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    node_connection1.stop(ctx).await?;

    inlet
        .update_outlet_node_route(ctx, route![node_connection2])
        .await?;

    let mut stream = TcpStream::connect(inlet.socket_address()).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    let res = handle.await;
    assert!(res.is_ok());

    Ok(())
}
