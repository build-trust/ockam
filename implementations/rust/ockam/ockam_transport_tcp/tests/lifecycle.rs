use core::time::Duration;
use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_transport_core::{Transport, TransportError};
use ockam_transport_tcp::{TcpTransport, TCP};
use tokio::net::TcpListener;

#[ockam_macros::test]
async fn test_resolve_address(ctx: &mut Context) -> Result<()> {
    let tcp = TcpTransport::create(ctx).await?;
    let tcp_address = "127.0.0.1:0";
    let initial_workers = ctx.list_workers().await?;
    let listener = TcpListener::bind(tcp_address)
        .await
        .map_err(TransportError::from)?;
    let local_address = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        // Accept two connections, sleep for 100ms and quit
        let (_stream1, _) = listener.accept().await.unwrap();
        let (_stream2, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    });

    let resolved = tcp
        .resolve_address(Address::new(TCP, local_address.clone()))
        .await?;

    // there are 2 additional workers
    let mut additional_workers = ctx.list_workers().await?;
    additional_workers.retain(|w| !initial_workers.contains(w));
    assert_eq!(additional_workers.len(), 2);

    // the TCP address is replaced with the TCP sender worker address
    assert!(additional_workers.contains(&resolved));

    // trying to resolve the address a second time should still work
    let _route = tcp
        .resolve_address(Address::new(TCP, local_address))
        .await?;

    tokio::time::sleep(Duration::from_millis(250)).await;

    Ok(())
}

#[ockam_macros::test]
async fn test_resolve_route_with_dns_address(ctx: &mut Context) -> Result<()> {
    let tcp = TcpTransport::create(ctx).await?;
    let tcp_address = "127.0.0.1:0";
    let listener = TcpListener::bind(tcp_address)
        .await
        .map_err(TransportError::from)?;
    let socket_address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        // Accept two connections, sleep for 100ms and quit
        let (_stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    // CI TRIGGER
    let result = tcp
        .resolve_address(Address::new(
            TCP,
            format!("localhost:{}", socket_address.port()),
        ))
        .await;
    assert!(result.is_ok());

    tokio::time::sleep(Duration::from_millis(250)).await;
    Ok(())
}
