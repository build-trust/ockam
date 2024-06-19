#![allow(dead_code)]

use crate::config::lookup::InternetAddress;
use crate::nodes::service::{NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions};
use ockam_node::{Context, HostnamePort, NodeBuilder};
use sqlx::__rt::timeout;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::runtime::Runtime;
use tracing::{error, info};

use ockam::identity::utils::AttributesBuilder;
use ockam::identity::SecureChannels;
use ockam::Result;
use ockam_core::AsyncTryClone;
use ockam_transport_tcp::{TcpListenerOptions, TcpTransport};

use crate::authenticator::credential_issuer::{DEFAULT_CREDENTIAL_VALIDITY, PROJECT_MEMBER_SCHEMA};
use crate::cli_state::{random_name, CliState};
use crate::nodes::service::{NodeManagerGeneralOptions, NodeManagerTransportOptions};
use crate::nodes::InMemoryNode;
use crate::nodes::{NodeManagerWorker, NODEMANAGER_ADDR};

/// This struct is used by tests, it has two responsibilities:
/// - guard to delete the cli state at the end of the test, the cli state
///   is comprised by some files within the file system, created in a
///   temporary directory, and possibly of sub-processes.
/// - useful access to the NodeManager
pub struct NodeManagerHandle {
    pub cli_state: CliState,
    pub node_manager: Arc<InMemoryNode>,
    pub tcp: TcpTransport,
    pub secure_channels: Arc<SecureChannels>,
}

impl Drop for NodeManagerHandle {
    fn drop(&mut self) {
        self.cli_state.delete().expect("cannot delete cli state");
    }
}

/// Starts a local node manager and returns a handle to it.
///
/// Be careful: if you drop the returned handle before the end of the test
/// things *will* break.
pub async fn start_manager_for_tests(
    context: &mut Context,
    bind_addr: Option<&str>,
    trust_options: Option<NodeManagerTrustOptions>,
) -> Result<NodeManagerHandle> {
    let tcp = TcpTransport::create(context).await?;
    let tcp_listener = tcp
        .listen(
            bind_addr.unwrap_or("127.0.0.1:0"),
            TcpListenerOptions::new(),
        )
        .await?;

    let cli_state = CliState::test().await?;

    let node_name = random_name();
    cli_state
        .start_node_with_optional_values(&node_name, &None, &None, Some(&tcp_listener))
        .await
        .unwrap();

    // Premise: we need an identity and a credential before the node manager starts.
    let identifier = cli_state.get_node(&node_name).await?.identifier();
    let named_vault = cli_state.get_or_create_default_named_vault().await?;
    let vault = cli_state.make_vault(named_vault).await?;
    let identities = cli_state.make_identities(vault).await?;

    let attributes = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA).build();
    let credential = identities
        .credentials()
        .credentials_creation()
        .issue_credential(
            &identifier,
            &identifier,
            attributes,
            DEFAULT_CREDENTIAL_VALIDITY,
        )
        .await
        .unwrap();

    let node_manager = InMemoryNode::new(
        context,
        NodeManagerGeneralOptions::new(cli_state.clone(), node_name, true, None, false),
        NodeManagerTransportOptions::new(
            tcp_listener.flow_control_id().clone(),
            tcp.async_try_clone().await?,
        ),
        trust_options.unwrap_or_else(|| {
            NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::InMemory(credential),
                NodeManagerCredentialRetrieverOptions::None,
                Some(identifier),
                NodeManagerCredentialRetrieverOptions::None,
            )
        }),
    )
    .await?;

    let node_manager = Arc::new(node_manager);
    let node_manager_worker = NodeManagerWorker::new(node_manager.clone());

    context
        .start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await?;

    let secure_channels = node_manager.secure_channels();
    let handle = NodeManagerHandle {
        cli_state,
        node_manager,
        tcp: tcp.async_try_clone().await?,
        secure_channels,
    };

    Ok(handle)
}

#[derive(Debug, Clone)]
pub struct EchoServerHandle {
    pub chosen_addr: HostnamePort,
    close: Arc<AtomicBool>,
}

impl Drop for EchoServerHandle {
    fn drop(&mut self) {
        self.close.store(true, Ordering::Relaxed);
    }
}

#[must_use = "listener closed when dropped"]
pub async fn start_tcp_echo_server() -> EchoServerHandle {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind server to address");

    let chosen_addr = listener.local_addr().unwrap();
    let close = Arc::new(AtomicBool::new(false));

    {
        let close = close.clone();
        tokio::spawn(async move {
            loop {
                let result = match timeout(Duration::from_millis(200), listener.accept()).await {
                    Ok(result) => result,
                    Err(_) => {
                        if close.load(Ordering::Relaxed) {
                            return;
                        }
                        continue;
                    }
                };

                let (mut socket, _) = result.expect("Failed to accept connection");
                socket.set_nodelay(true).unwrap();

                tokio::spawn(async move {
                    let mut buf = vec![0; 1024];
                    loop {
                        let n = match socket.read(&mut buf).await {
                            // socket closed
                            Ok(0) => return,
                            Ok(n) => n,
                            Err(e) => {
                                println!("Failed to read from socket; err = {:?}", e);
                                return;
                            }
                        };

                        // Write the data back
                        if let Err(e) = socket.write_all(&buf[0..n]).await {
                            println!("Failed to write to socket; err = {:?}", e);
                            return;
                        }
                    }
                });
            }
        });
    }

    EchoServerHandle {
        chosen_addr: HostnamePort::from_socket_addr(chosen_addr),
        close,
    }
}

pub struct TestNode {
    pub context: Context,
    pub node_manager_handle: NodeManagerHandle,
}

impl TestNode {
    pub async fn create(runtime: Arc<Runtime>, listen_addr: Option<&str>) -> Self {
        let (mut context, mut executor) = NodeBuilder::new().with_runtime(runtime.clone()).build();
        runtime.spawn(async move {
            executor.start_router().await.expect("cannot start router");
        });
        let node_manager_handle = start_manager_for_tests(
            &mut context,
            listen_addr,
            Some(NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::None,
                NodeManagerCredentialRetrieverOptions::None,
                None,
                NodeManagerCredentialRetrieverOptions::None,
            )),
        )
        .await
        .expect("cannot start node manager");

        Self {
            context,
            node_manager_handle,
        }
    }

    pub async fn listen_address(&self) -> InternetAddress {
        self.cli_state
            .get_node(&self.node_manager.node_name())
            .await
            .unwrap()
            .tcp_listener_address()
            .unwrap()
    }
}

impl Deref for TestNode {
    type Target = NodeManagerHandle;

    fn deref(&self) -> &Self::Target {
        &self.node_manager_handle
    }
}

pub struct PassthroughServerHandle {
    pub chosen_addr: SocketAddr,
    pub destination: SocketAddr,
    close: Arc<AtomicBool>,
}

impl Drop for PassthroughServerHandle {
    fn drop(&mut self) {
        self.close.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
pub enum Disruption {
    None,
    LimitBandwidth(usize),
    DropPacketsAfter(usize),
    PacketsOutOfOrderAfter(usize),
}

#[must_use = "listener closed when dropped"]
pub async fn start_passthrough_server(
    destination: &str,
    outgoing_disruption: Disruption,
    incoming_disruption: Disruption,
) -> PassthroughServerHandle {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let socket = TcpSocket::new_v4().unwrap();

    // to reduce the impact of this passthrough server on the benchmarks and tests
    // we set the receive buffer size to 1KB
    socket.set_recv_buffer_size(1024).unwrap();
    socket.bind(addr).expect("Failed to bind server to address");

    let listener = socket.listen(32).unwrap();

    let destination = destination
        .parse()
        .expect("Failed to parse destination address");

    let chosen_addr = listener.local_addr().unwrap();
    let close = Arc::new(AtomicBool::new(false));

    {
        let close = close.clone();
        tokio::spawn(async move {
            loop {
                let result = match timeout(Duration::from_millis(200), listener.accept()).await {
                    Ok(result) => result,
                    Err(_) => {
                        if close.load(Ordering::Relaxed) {
                            return;
                        }
                        continue;
                    }
                };

                let (incoming_socket, _) = result.expect("Failed to accept connection");
                tokio::spawn(async move {
                    let outgoing_socket = match TcpStream::connect(destination).await {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to connect to destination; err = {:?}", e);
                            return;
                        }
                    };
                    let (incoming_read, incoming_write) = incoming_socket.into_split();
                    let (outgoing_read, outgoing_write) = outgoing_socket.into_split();

                    start_relay_for(outgoing_disruption, incoming_read, outgoing_write);
                    start_relay_for(incoming_disruption, outgoing_read, incoming_write);
                });
            }
        });
    }

    PassthroughServerHandle {
        chosen_addr,
        destination,
        close,
    }
}

fn start_relay_for(disruption: Disruption, read: OwnedReadHalf, write: OwnedWriteHalf) {
    match disruption {
        Disruption::None => {
            tokio::spawn(async move { relay_stream_limit_bandwidth(read, write, None).await });
        }
        Disruption::LimitBandwidth(bytes_per_second) => {
            tokio::spawn(async move {
                relay_stream_limit_bandwidth(read, write, Some(bytes_per_second)).await
            });
        }
        Disruption::DropPacketsAfter(drop_packets_after) => {
            tokio::spawn(async move {
                relay_stream_drop_packets(read, write, drop_packets_after).await
            });
        }
        Disruption::PacketsOutOfOrderAfter(packet_out_of_order_after) => {
            tokio::spawn(async move {
                relay_stream_packets_out_of_order(read, write, packet_out_of_order_after).await
            });
        }
    }
}

async fn relay_stream_limit_bandwidth(
    mut read_half: OwnedReadHalf,
    mut write_half: OwnedWriteHalf,
    max_bytes_per_second: Option<usize>,
) {
    let mut bytes_counter = 0;
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let read = match read_half.read(&mut buffer).await {
            // socket closed
            Ok(0) => return,
            Ok(n) => n,
            Err(e) => {
                error!("Failed to read from socket; err = {:?}", e);
                return;
            }
        };

        if let Err(e) = write_half.write_all(&buffer[0..read]).await {
            error!("Failed to write to socket; err = {:?}", e);
            return;
        }

        bytes_counter += read;
        if let Some(max_bytes_per_second) = max_bytes_per_second {
            let nanoseconds =
                (1_000_000_000f32 * (bytes_counter as f32 / max_bytes_per_second as f32)) as u64;
            tokio::time::sleep(Duration::from_nanos(nanoseconds)).await;
            bytes_counter = 0;
        }
    }
}

#[allow(unused)]
async fn relay_stream_drop_packets(
    mut read_half: OwnedReadHalf,
    mut write_half: OwnedWriteHalf,
    drop_packets_after: usize,
) {
    let mut packet_counter: usize = 0;
    let mut buffer = vec![0; 64 * 1024];
    loop {
        // read the first 2 bytes with the packet size
        match read_half.read_exact(&mut buffer[0..2]).await {
            // socket closed
            Ok(0) => return,
            Ok(n) => {
                if n != 2 {
                    error!(
                        "Failed to read from socket; err = {:?}",
                        "incomplete packet size"
                    );
                    return;
                }
            }
            Err(e) => {
                error!("Failed to read from socket; err = {:?}", e);
                return;
            }
        };

        let packet_size = (&buffer[0..2]).read_u16().await.unwrap() + 2;
        match read_half
            .read_exact(&mut buffer[2..packet_size as usize])
            .await
        {
            // socket closed
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                error!("Failed to read from socket; err = {:?}", e);
                return;
            }
        }

        if packet_counter <= drop_packets_after || packet_counter % 2 == 0 {
            if let Err(e) = write_half.write_all(&buffer[0..packet_size as usize]).await {
                error!("Failed to write to socket; err = {:?}", e);
                return;
            }
        } else {
            info!("Dropping packet {packet_counter} of size {packet_size}");
        }

        packet_counter += 1;
    }
}

async fn relay_stream_packets_out_of_order(
    mut read_half: OwnedReadHalf,
    mut write_half: OwnedWriteHalf,
    packet_out_of_order_after: usize,
) {
    let mut packet_counter: usize = 0;
    let mut previus_buffer: Option<Vec<u8>> = None;
    let mut buffer = vec![0; 64 * 1024];
    loop {
        // read the first 2 bytes with the packet size
        match read_half.read_exact(&mut buffer[0..2]).await {
            // socket closed
            Ok(0) => return,
            Ok(n) => {
                if n != 2 {
                    error!(
                        "Failed to read from socket; err = {:?}",
                        "incomplete packet size"
                    );
                    return;
                }
            }
            Err(e) => {
                error!("Failed to read from socket; err = {:?}", e);
                return;
            }
        };

        let packet_size = (&buffer[0..2]).read_u16().await.unwrap() + 2;
        match read_half
            .read_exact(&mut buffer[2..packet_size as usize])
            .await
        {
            // socket closed
            Ok(0) => return,
            Ok(_) => {}
            Err(e) => {
                error!("Failed to read from socket; err = {:?}", e);
                return;
            }
        };

        if packet_counter > packet_out_of_order_after {
            // write the packet and then the previous one
            if packet_counter % 2 == 0 {
                if let Err(e) = write_half.write_all(&buffer[0..packet_size as usize]).await {
                    error!("Failed to write to socket; err = {:?}", e);
                    return;
                }

                if let Some(previous_buffer) = previus_buffer.take() {
                    if let Err(e) = write_half.write_all(&previous_buffer).await {
                        error!("Failed to write to socket; err = {:?}", e);
                        return;
                    }
                }
            } else {
                info!("Reversing order of packet {packet_counter} of size {packet_size}");
                previus_buffer = Some(buffer[0..packet_size as usize].to_vec());
            }
        } else if let Err(e) = write_half.write_all(&buffer[0..packet_size as usize]).await {
            error!("Failed to write to socket; err = {:?}", e);
            return;
        }

        packet_counter += 1;
    }
}
