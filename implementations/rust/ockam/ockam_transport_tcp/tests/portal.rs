use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use ockam_core::compat::rand::{self, random, Rng};
use ockam_core::{route, Result};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

const LENGTH: usize = 32;

async fn setup(ctx: &Context) -> Result<(String, TcpListener)> {
    let gen_bind_addr = || {
        let rand_port = rand::thread_rng().gen_range(1024..65535);
        format!("127.0.0.1:{}", rand_port)
    };
    let bind_address;

    let listener;
    loop {
        let try_bind_addr = gen_bind_addr();
        if let Ok(l) = TcpListener::bind(&try_bind_addr).await {
            listener = l;
            bind_address = try_bind_addr;
            break;
        }
    }

    let tcp = TcpTransport::create(ctx).await?;

    tcp.create_outlet("outlet", bind_address).await?;

    let inlet_addr;
    loop {
        let try_bind_addr = gen_bind_addr();
        if tcp
            .create_inlet(&try_bind_addr, route!["outlet"])
            .await
            .is_ok()
        {
            inlet_addr = try_bind_addr;
            break;
        }
    }

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
    Ok(())
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
    Ok(())
}
