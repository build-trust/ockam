use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use ockam_core::compat::rand::random;
use ockam_core::{route, LocalSourceOnly, Result};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

const LENGTH: usize = 32;

async fn setup(ctx: &Context) -> Result<(String, TcpListener)> {
    let tcp = TcpTransport::create(ctx).await?;

    let listener = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_address = listener.local_addr().unwrap().to_string();
        tcp.create_outlet("outlet", bind_address.clone(), Arc::new(LocalSourceOnly))
            .await?;
        listener
    };

    let (_, inlet_saddr) = tcp
        .create_inlet("127.0.0.1:0", route!["outlet"], Arc::new(LocalSourceOnly))
        .await?;

    Ok((inlet_saddr.to_string(), listener))
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

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

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

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }

    Ok(())
}
