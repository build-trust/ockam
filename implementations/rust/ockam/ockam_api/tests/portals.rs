use ockam_api::config::lookup::InternetAddress;
use ockam_api::nodes::models::portal::OutletAccessControl;
use ockam_api::test_utils::{
    start_manager_for_tests, start_passthrough_server, start_tcp_echo_server, Disruption, TestNode,
};
use ockam_api::ConnectionStatus;
use ockam_core::compat::rand::RngCore;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Address, AllowAll, Error};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::time::timeout;
use tracing::info;

#[ockam_macros::test]
async fn inlet_outlet_local_successful(context: &mut Context) -> ockam::Result<()> {
    let echo_server_handle = start_tcp_echo_server().await;
    let node_manager_handle = start_manager_for_tests(context, None, None).await?;

    let outlet_status = node_manager_handle
        .node_manager
        .create_outlet(
            context,
            echo_server_handle.chosen_addr.clone(),
            false,
            Some(Address::from_string("outlet")),
            true,
            OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
        )
        .await?;

    assert_eq!(
        outlet_status.socket_addr,
        echo_server_handle.chosen_addr.to_socket_addr()?
    );
    assert_eq!(outlet_status.worker_addr.address(), "outlet");

    let inlet_status = node_manager_handle
        .node_manager
        .create_inlet(
            context,
            "127.0.0.1:0".to_string(),
            route![],
            route![],
            MultiAddr::from_str("/secure/api/service/outlet")?,
            "alias".to_string(),
            None,
            None,
            None,
            true,
            None,
            false,
            false,
        )
        .await?;

    assert_eq!(inlet_status.alias, "alias");
    assert_eq!(inlet_status.status, ConnectionStatus::Up);
    assert_eq!(inlet_status.outlet_addr, "/secure/api/service/outlet");
    assert_ne!(inlet_status.bind_addr, "127.0.0.1:0");
    assert!(inlet_status.outlet_route.is_some());

    // connect to inlet_status.bind_addr and send dummy payload
    let mut socket = TcpStream::connect(inlet_status.bind_addr).await.unwrap();
    socket.write_all(b"hello").await.unwrap();

    let mut buf = [0u8; 5];
    socket.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"hello");

    Ok(())
}

#[test]
fn portal_node_goes_down_reconnect() {
    // in this test we manually create three nodes with a shared runtime, then:
    //  - create a portal using the first two nodes
    //  - bring down the second node
    //  - verify that's detected as offline
    //  - create a third node using the same address as the second one
    //  - verify the portal is restored

    let runtime = Arc::new(Runtime::new().unwrap());
    let handle = runtime.handle();
    let runtime_cloned = runtime.clone();
    std::env::set_var("OCKAM_LOG", "none");

    let result: ockam::Result<()> = handle.block_on(async move {
        let test_body = async move {
            let echo_server_handle = start_tcp_echo_server().await;

            let first_node = TestNode::create(runtime_cloned.clone(), None).await;
            let second_node = TestNode::create(runtime_cloned.clone(), None).await;

            let _outlet_status = second_node
                .node_manager
                .create_outlet(
                    &second_node.context,
                    echo_server_handle.chosen_addr.clone(),
                    false,
                    Some(Address::from_string("outlet")),
                    true,
                    OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
                )
                .await?;

            let second_node_listen_address = second_node.listen_address().await;

            // create inlet in the first node pointing to the second one
            let inlet_status = first_node
                .node_manager
                .create_inlet(
                    &first_node.context,
                    "127.0.0.1:0".to_string(),
                    route![],
                    route![],
                    second_node_listen_address
                        .multi_addr()?
                        .concat(&MultiAddr::from_str("/secure/api/service/outlet")?)?,
                    "inlet_alias".to_string(),
                    None,
                    None,
                    None,
                    true,
                    None,
                    false,
                    false,
                )
                .await?;

            // connect to inlet_status.bind_addr and send dummy payload
            let mut socket = TcpStream::connect(inlet_status.bind_addr.clone())
                .await
                .unwrap();
            socket.write_all(b"hello").await.unwrap();

            let mut buf = [0u8; 5];
            socket.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hello");

            second_node.context.stop().await?;

            // now let's verify the inlet has been detected as down
            loop {
                let inlet_status = first_node
                    .node_manager
                    .show_inlet("inlet_alias")
                    .await
                    .unwrap();
                if inlet_status.status == ConnectionStatus::Down {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5000)).await;
            }

            // create third node using the same address as the second one
            let third_node = TestNode::create(
                runtime_cloned,
                Some(&second_node_listen_address.to_string()),
            )
            .await;

            let _outlet_status = third_node
                .node_manager
                .create_outlet(
                    &third_node.context,
                    echo_server_handle.chosen_addr.clone(),
                    false,
                    Some(Address::from_string("outlet")),
                    true,
                    OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
                )
                .await?;

            // now let's verify the inlet has been restored
            loop {
                let inlet_status = first_node
                    .node_manager
                    .show_inlet("inlet_alias")
                    .await
                    .unwrap();
                if inlet_status.status == ConnectionStatus::Up {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5000)).await;
            }

            let mut socket = TcpStream::connect(inlet_status.bind_addr).await.unwrap();
            socket.write_all(b"hello").await.unwrap();

            let mut buf = [0u8; 5];
            socket.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hello");

            third_node.context.stop().await?;
            first_node.context.stop().await?;

            Ok(())
        };

        timeout(Duration::from_secs(90), test_body)
            .await
            .unwrap_or_else(|_| Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out")))
    });

    result.unwrap();
}

#[test]
fn portal_low_bandwidth_connection_keep_working_for_60s() {
    // in this test we use two nodes, connected through a passthrough server
    // which limits the bandwidth to 64kb per second
    //
    // ┌────────┐     ┌───────────┐        ┌────────┐
    // │  Node  └─────►    TCP    └────────►  Node  │
    // │   1    ◄─────┐Passthrough◄────────┐   2    │
    // └────┬───┘     │  64KB/s   │        └────▲───┘
    //      │         └───────────┘             │
    //      │         ┌───────────┐             │
    //      │ Portal  │   TCP     │      Outlet │
    //      └─────────┤   Echo    ◄─────────────┘
    //                └───────────┘

    let runtime = Arc::new(Runtime::new().unwrap());
    let handle = runtime.handle();
    let runtime_cloned = runtime.clone();
    std::env::set_var("OCKAM_LOG", "none");

    let result: ockam::Result<()> = handle.block_on(async move {
        let test_body = async move {
            let echo_server_handle = start_tcp_echo_server().await;

            let first_node = TestNode::create(runtime_cloned.clone(), None).await;
            let second_node = TestNode::create(runtime_cloned, None).await;

            let _outlet_status = second_node
                .node_manager
                .create_outlet(
                    &second_node.context,
                    echo_server_handle.chosen_addr.clone(),
                    false,
                    Some(Address::from_string("outlet")),
                    true,
                    OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
                )
                .await?;

            let second_node_listen_address = second_node
                .cli_state
                .get_node(&second_node.node_manager.node_name())
                .await?
                .tcp_listener_address()
                .unwrap();

            let passthrough_server_handle = start_passthrough_server(
                &second_node_listen_address.to_string(),
                Disruption::LimitBandwidth(64 * 1024),
                Disruption::LimitBandwidth(64 * 1024),
            )
            .await;

            // create inlet in the first node pointing to the second one
            let inlet_status = first_node
                .node_manager
                .create_inlet(
                    &first_node.context,
                    "127.0.0.1:0".to_string(),
                    route![],
                    route![],
                    InternetAddress::from(passthrough_server_handle.chosen_addr)
                        .multi_addr()?
                        .concat(&MultiAddr::from_str("/secure/api/service/outlet")?)?,
                    "inlet_alias".to_string(),
                    None,
                    None,
                    None,
                    true,
                    None,
                    false,
                    false,
                )
                .await?;

            info!("inlet_status: {inlet_status:?}");

            // connect to inlet_status.bind_addr and send dummy payload
            let mut buf = [0u8; 48 * 1024];
            let mut stream = TcpStream::connect(inlet_status.bind_addr.clone())
                .await
                .unwrap();

            // check that the route is up
            stream.write_all(b"hello").await.unwrap();
            stream.read_exact(&mut buf[0..5]).await.unwrap();
            assert_eq!(&buf[0..5], b"hello");

            // saturate the bandwidth with empty packets
            // and verify the connection stays up for 30 seconds
            let end = std::time::Instant::now() + Duration::from_secs(60);
            let (mut rx, mut tx) = stream.into_split();

            spawn(async move {
                while std::time::Instant::now() < end {
                    let _ = tx.write_all(&buf).await;
                }
            });

            spawn(async move {
                while std::time::Instant::now() < end {
                    let _ = rx.read(&mut buf).await.unwrap();
                }
            });

            // keep checking the status of the inlet
            while std::time::Instant::now() < end {
                let inlet_status = first_node
                    .node_manager
                    .show_inlet("inlet_alias")
                    .await
                    .unwrap();
                assert_eq!(inlet_status.status, ConnectionStatus::Up);
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }

            second_node.context.stop().await?;
            first_node.context.stop().await?;

            Ok(())
        };

        timeout(Duration::from_secs(90), test_body)
            .await
            .unwrap_or_else(|_| Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out")))
    });

    result.unwrap();
}

#[test]
fn portal_heavy_load_exchanged() {
    let runtime = Arc::new(Runtime::new().unwrap());
    let handle = runtime.handle();
    let runtime_cloned = runtime.clone();
    std::env::set_var("OCKAM_LOG", "none");

    let result: ockam::Result<()> = handle.block_on(async move {
        let test_body = async move {
            let echo_server_handle = start_tcp_echo_server().await;

            let first_node = TestNode::create(runtime_cloned.clone(), None).await;
            let second_node = TestNode::create(runtime_cloned, None).await;

            let _outlet_status = second_node
                .node_manager
                .create_outlet(
                    &second_node.context,
                    echo_server_handle.chosen_addr.clone(),
                    false,
                    Some(Address::from_string("outlet")),
                    true,
                    OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
                )
                .await?;

            let second_node_listen_address = second_node
                .cli_state
                .get_node(&second_node.node_manager.node_name())
                .await?
                .tcp_listener_address()
                .unwrap();

            // create inlet in the first node pointing to the second one
            let inlet_status = first_node
                .node_manager
                .create_inlet(
                    &first_node.context,
                    "127.0.0.1:0".to_string(),
                    route![],
                    route![],
                    second_node_listen_address
                        .multi_addr()?
                        .concat(&MultiAddr::from_str("/secure/api/service/outlet")?)?,
                    "inlet_alias".to_string(),
                    None,
                    None,
                    None,
                    true,
                    None,
                    false,
                    false,
                )
                .await?;

            info!("inlet_status: {inlet_status:?}");

            // connect to inlet_status.bind_addr and send dummy payload
            const PAYLOAD_SIZE: usize = 50 * 1024 * 1024;
            info!("generating random payload");
            let payload = {
                let mut payload = vec![0u8; PAYLOAD_SIZE];
                rand::thread_rng().fill_bytes(&mut payload);
                payload
            };

            let stream = TcpStream::connect(inlet_status.bind_addr.clone())
                .await
                .unwrap();

            // saturate the bandwidth with empty packets
            // and verify the connection stays up for 30 seconds
            let (mut rx, mut tx) = stream.into_split();

            let payload_cloned = payload.clone();
            // keeps a reference to the connection to avoid it being dropped
            let join_tx = spawn(async move {
                info!("writing payload");
                tx.write_all(&payload_cloned).await.unwrap();
                info!("payload fully written");
                tx
            });

            let mut incoming_buffer = vec![0; PAYLOAD_SIZE];
            let size = rx.read_exact(incoming_buffer.as_mut_slice()).await.unwrap();

            // check that data is correct up to the size
            assert_eq!(payload.len(), size);

            // using assert!() to avoid MB of data being shown in the logs
            assert!(payload == incoming_buffer);

            let _ = join_tx.await.unwrap();
            second_node.context.stop().await?;
            first_node.context.stop().await?;

            Ok(())
        };

        timeout(Duration::from_secs(90), test_body)
            .await
            .unwrap_or_else(|_| Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out")))
    });

    result.unwrap();
}

#[ignore]
#[test]
fn portal_connection_drop_packets() {
    // Drop even packets after 32 packets (to allow for the initial
    // handshake to complete).
    // This test checks that:
    //   - connection is interrupted when a failure is detected
    //   - the portion of the received data matches with the sent data.
    //

    test_portal_payload_transfer(Disruption::DropPacketsAfter(32), Disruption::None);
}

#[ignore]
#[test]
fn portal_connection_change_packet_order() {
    // Change packet order after 32 packets (to allow for the initial
    // handshake to complete).
    // This test checks that:
    //   - connection is interrupted when a failure is detected
    //   - the portion of the received data matches with the sent data.

    test_portal_payload_transfer(Disruption::PacketsOutOfOrderAfter(32), Disruption::None);
}

fn test_portal_payload_transfer(outgoing_disruption: Disruption, incoming_disruption: Disruption) {
    // we use two nodes, connected through a passthrough server
    // ┌────────┐     ┌───────────┐        ┌────────┐
    // │  Node  └─────►    TCP    └────────►  Node  │
    // │   1    ◄─────┐Passthrough◄────────┐   2    │
    // └────┬───┘     │ Disruption│        └────▲───┘
    //      │         └───────────┘             │
    //      │         ┌───────────┐             │
    //      │ Portal  │   TCP     │      Outlet │
    //      └─────────┤   Echo    ◄─────────────┘
    //                └───────────┘

    let runtime = Arc::new(Runtime::new().unwrap());
    let handle = runtime.handle();
    let runtime_cloned = runtime.clone();
    std::env::set_var("OCKAM_LOG", "none");

    let result: ockam::Result<_> = handle.block_on(async move {
        let test_body = async move {
            let echo_server_handle = start_tcp_echo_server().await;

            let first_node = TestNode::create(runtime_cloned.clone(), None).await;
            let second_node = TestNode::create(runtime_cloned, None).await;

            let _outlet_status = second_node
                .node_manager
                .create_outlet(
                    &second_node.context,
                    echo_server_handle.chosen_addr.clone(),
                    false,
                    Some(Address::from_string("outlet")),
                    true,
                    OutletAccessControl::AccessControl((Arc::new(AllowAll), Arc::new(AllowAll))),
                )
                .await?;

            let second_node_listen_address = second_node
                .cli_state
                .get_node(&second_node.node_manager.node_name())
                .await?
                .tcp_listener_address()
                .unwrap();

            let passthrough_server_handle = start_passthrough_server(
                &second_node_listen_address.to_string(),
                outgoing_disruption,
                incoming_disruption,
            )
            .await;

            // create inlet in the first node pointing to the second one
            let inlet_status = first_node
                .node_manager
                .create_inlet(
                    &first_node.context,
                    "127.0.0.1:0".to_string(),
                    route![],
                    route![],
                    InternetAddress::from(passthrough_server_handle.chosen_addr)
                        .multi_addr()?
                        .concat(&MultiAddr::from_str("/secure/api/service/outlet")?)?,
                    "inlet_alias".to_string(),
                    None,
                    None,
                    None,
                    true,
                    None,
                    false,
                    false,
                )
                .await?;

            info!("inlet_status: {inlet_status:?}");

            // send 10MB of random data and verify it's correct on the other side
            let payload_size = 10 * 1024 * 1024;
            let mut random_buffer: Vec<u8> = vec![0; payload_size];
            rand::thread_rng().fill_bytes(&mut random_buffer);

            // connect to inlet_status.bind_addr and send dummy payload
            let stream = TcpStream::connect(inlet_status.bind_addr.clone())
                .await
                .unwrap();

            // we can't send and read the data from a sigle async context
            let (mut rx, mut tx) = stream.into_split();

            let copied_buffer = random_buffer.clone();
            let _join = spawn(async move {
                let _ = tx.write_all(&copied_buffer).await;
                tx
            });

            let mut incoming_buffer = Vec::new();

            // this call keep reading the buffer until the connection is closed
            // we are validating that the connection actually gets closed.
            // since the connection can be asbruptly closed, we ignore errors
            let _ = rx.read_to_end(&mut incoming_buffer).await;
            let size = incoming_buffer.len();

            info!("size: {}", size);
            assert_ne!(size, 0);
            assert!(size < payload_size);

            // check that data is correct up to the size,
            // using assert to avoid MB of data being shown in the logs
            assert!(random_buffer[0..size] == incoming_buffer[0..size]);

            second_node.context.stop().await?;
            first_node.context.stop().await?;

            Ok(())
        };

        timeout(Duration::from_secs(60), test_body)
            .await
            .unwrap_or_else(|_| Err(Error::new(Origin::Node, Kind::Timeout, "Test timed out")))
    });

    result.unwrap();
}
