use ockam_core::{try_route, Result};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use rand::{random, Rng};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const LENGTH: usize = 32;

async fn setup(ctx: &Context) -> Result<(String, TcpListener)> {
    let target_port = rand::thread_rng().gen_range(10000, 65535);
    let target_addr = format!("127.0.0.1:{}", target_port);
    let listener = TcpListener::bind(target_addr.clone()).await.unwrap();

    let tcp = TcpTransport::create(ctx).await?;

    tcp.create_outlet("outlet", target_addr).await?;

    let inlet_port = rand::thread_rng().gen_range(10000, 65535);
    let inlet_addr = format!("127.0.0.1:{}", inlet_port);
    tcp.create_inlet(inlet_addr.clone(), try_route!["outlet"]?)
        .await?;

    Ok((inlet_addr, listener))
}

fn generate_binary() -> [u8; LENGTH] {
    random()
}

async fn write_binary(stream: &mut TcpStream, payload: [u8; LENGTH]) {
    stream.write_all(&payload).await.unwrap();
}

async fn read_assert_binary(stream: &mut TcpStream, expected_payload: [u8; LENGTH]) {
    let mut payload = [0u8; LENGTH];
    let length = stream.read(&mut payload).await.unwrap();
    assert_eq!(length, LENGTH);
    assert_eq!(payload, expected_payload);
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn portal__standard_flow__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let (inlet_addr, listener) = setup(ctx).await?;

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        read_assert_binary(&mut stream, payload1).await;
        write_binary(&mut stream, payload2).await;
    });

    // Wait till listener is up
    tokio::time::sleep(Duration::new(0, 250_000)).await;

    let mut stream = TcpStream::connect(inlet_addr).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    tokio::time::sleep(Duration::new(0, 250_000)).await;

    ctx.stop().await
}

#[allow(non_snake_case)]
#[ockam_macros::test(timeout = 5000)]
async fn portal__reverse_flow__should_succeed(ctx: &mut Context) -> Result<()> {
    let payload1 = generate_binary();
    let payload2 = generate_binary();

    let (inlet_addr, listener) = setup(ctx).await?;

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        write_binary(&mut stream, payload2).await;
        read_assert_binary(&mut stream, payload1).await;
    });

    // Wait till listener is up
    tokio::time::sleep(Duration::new(0, 250_000)).await;

    let mut stream = TcpStream::connect(inlet_addr).await.unwrap();
    write_binary(&mut stream, payload1).await;
    read_assert_binary(&mut stream, payload2).await;

    tokio::time::sleep(Duration::new(0, 250_000)).await;

    ctx.stop().await
}
