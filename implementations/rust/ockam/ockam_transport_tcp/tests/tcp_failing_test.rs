use ockam_core::{Address, Result};
use ockam_node::NodeBuilder;
use ockam_transport_core::Transport;
use ockam_transport_tcp::{TcpTransport, TCP};

#[test]
fn test_resolve_route_google() -> Result<()> {
    for _ in 0..100 {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                let tcp = TcpTransport::create(&ctx).await?;

                let result = tcp
                    .resolve_address(Address::new(TCP, "google.com:80"))
                    .await;
                assert!(result.is_ok());

                ctx.stop().await?;

                Ok(())
            })
            .unwrap()
            .unwrap()
    }

    Ok(())
}

#[test]
fn test_resolve_route_google2() -> Result<()> {
    for _ in 0..100 {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                let tcp = TcpTransport::create(&ctx).await?;

                let result = tcp
                    .resolve_address(Address::new(TCP, "google.com:80"))
                    .await;
                assert!(result.is_ok());

                ctx.stop().await?;

                Ok(())
            })
            .unwrap()
            .unwrap()
    }

    Ok(())
}
