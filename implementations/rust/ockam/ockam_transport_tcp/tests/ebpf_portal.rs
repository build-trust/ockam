#[cfg(privileged_portals_support)]
mod tests {
    use log::info;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use ockam_core::compat::rand::random;
    use ockam_core::{route, Result};
    use ockam_node::Context;
    use ockam_transport_tcp::{TcpInletOptions, TcpOutletOptions, TcpTransport};

    const LENGTH: usize = 32;

    async fn setup(tcp: &TcpTransport) -> Result<(String, TcpListener)> {
        let listener = {
            let listener = TcpListener::bind("localhost:0").await.unwrap();

            let bind_address = listener.local_addr().unwrap().to_string();
            info!("Listener address: {}", bind_address);
            tcp.create_privileged_outlet(
                "outlet",
                bind_address.try_into().unwrap(),
                TcpOutletOptions::new(),
            )
            .await?;
            listener
        };

        let inlet = tcp
            .create_privileged_inlet("localhost:0", route!["outlet"], TcpInletOptions::new())
            .await?;

        let inlet_address = inlet.socket_address().to_string();

        info!("Inlet address: {}", inlet_address);

        Ok((inlet_address, listener))
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

    #[allow(non_snake_case)]
    #[ockam_macros::test(timeout = 5000)]
    #[ignore] // Requires root and capabilities
    async fn privileged_portal__standard_flow__should_succeed(ctx: &mut Context) -> Result<()> {
        let tcp = TcpTransport::create(ctx)?;

        let payload1 = generate_binary();
        let payload2 = generate_binary();

        let (inlet_addr, listener) = setup(&tcp).await?;

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
}
