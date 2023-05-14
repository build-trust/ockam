use std::net::{SocketAddrV4, SocketAddrV6};

use anyhow::anyhow;

use ockam::TcpTransport;
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Error, Result, Route, TransportType, LOCAL};
use ockam_multiaddr::proto::{
    DnsAddr, Ip4, Ip6, Node, Project, Secure, Service, Space, Tcp, Worker,
};
use ockam_multiaddr::{Code, MultiAddr, Protocol};
use ockam_transport_tcp::{TcpConnectionOptions, TCP};

use crate::error::ApiError;

/// Try to convert a multi-address to an Ockam route.
pub fn local_multiaddr_to_route(ma: &MultiAddr) -> Option<Route> {
    let mut rb = Route::new();
    for p in ma.iter() {
        match p.code() {
            // Only hops that are directly translated to existing workers are allowed here
            Worker::CODE => {
                let local = p.cast::<Worker>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }

            // If your code crashes here then the front-end CLI isn't
            // properly calling `clean_multiaddr` before passing it to
            // the backend
            Node::CODE => unreachable!(),
            Ip4::CODE | Ip6::CODE | DnsAddr::CODE => unreachable!(),

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(rb.into())
}

pub struct MultiAddrToRouteResult {
    pub flow_control_id: Option<FlowControlId>,
    pub route: Route,
    pub tcp_worker: Option<Address>,
}

pub async fn multiaddr_to_route(
    ma: &MultiAddr,
    tcp: &TcpTransport,
) -> Option<MultiAddrToRouteResult> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();

    let mut flow_control_id = None;
    let mut number_of_tcp_hops = 0;
    let mut tcp_worker = None;

    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let ip4 = p.cast::<Ip4>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV4::new(*ip4, *port);

                let options = TcpConnectionOptions::new();
                flow_control_id = Some(options.producer_flow_control_id().clone());

                let addr = tcp
                    .connect(socket_addr.to_string(), options)
                    .await
                    .ok()?
                    .sender_address()
                    .clone();
                tcp_worker = Some(addr.clone());

                number_of_tcp_hops += 1;
                rb = rb.append(addr)
            }
            Ip6::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let ip6 = p.cast::<Ip6>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);

                let options = TcpConnectionOptions::new();
                flow_control_id = Some(options.producer_flow_control_id().clone());

                let addr = tcp
                    .connect(socket_addr.to_string(), options)
                    .await
                    .ok()?
                    .sender_address()
                    .clone();
                tcp_worker = Some(addr.clone());

                number_of_tcp_hops += 1;
                rb = rb.append(addr)
            }
            DnsAddr::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let port = p.cast::<Tcp>()?;

                        let options = TcpConnectionOptions::new();
                        flow_control_id = Some(options.producer_flow_control_id().clone());

                        let addr = tcp
                            .connect(format!("{}:{}", &*host, *port), options)
                            .await
                            .ok()?
                            .sender_address()
                            .clone();
                        tcp_worker = Some(addr.clone());

                        number_of_tcp_hops += 1;
                        rb = rb.append(addr);
                        let _ = it.next();
                        continue;
                    }
                }
            }
            Worker::CODE => {
                let local = p.cast::<Worker>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }

    Some(MultiAddrToRouteResult {
        flow_control_id,
        tcp_worker,
        route: rb.into(),
    })
}

/// Resolve all the multiaddresses which represent transport addresses
/// For example /tcp/127.0.0.1/port/4000 is transformed to the Address (TCP, "127.0.0.1:4000")
/// The creation of a TCP worker and the substitution of that transport address to a worker address
/// is done later with `context.resolve_transport_route(route)`
pub fn multiaddr_to_transport_route(ma: &MultiAddr) -> Option<Route> {
    let mut route = Route::new();
    let mut it = ma.iter().peekable();

    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                let ip4 = p.cast::<Ip4>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV4::new(*ip4, *port);
                route = route.append(Address::new(TCP, socket_addr.to_string()))
            }
            Ip6::CODE => {
                let ip6 = p.cast::<Ip6>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);
                route = route.append(Address::new(TransportType::new(1), socket_addr.to_string()))
            }
            DnsAddr::CODE => {
                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let port = p.cast::<Tcp>()?;
                        let addr = format!("{}:{}", &*host, *port);
                        route = route.append(Address::new(TransportType::new(1), addr));
                        let _ = it.next();
                        continue;
                    }
                }
            }
            Worker::CODE => {
                let local = p.cast::<Worker>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(route.into())
}

/// Try to convert a multiaddr to an Ockam Address
pub fn multiaddr_to_addr(ma: &MultiAddr) -> Option<Address> {
    let mut it = ma.iter().peekable();
    let p = it.next()?;
    match p.code() {
        Worker::CODE => {
            let local = p.cast::<Worker>()?;
            Some(Address::new(LOCAL, &*local))
        }
        Service::CODE => {
            let local = p.cast::<Service>()?;
            Some(Address::new(LOCAL, &*local))
        }
        _ => None,
    }
}

pub fn try_multiaddr_to_addr(ma: &MultiAddr) -> Result<Address, Error> {
    multiaddr_to_addr(ma)
        .ok_or_else(|| ApiError::message(format!("could not convert {ma} to address")))
}

/// Try to convert an Ockam Route into a MultiAddr.
pub fn route_to_multiaddr(r: &Route) -> Option<MultiAddr> {
    let mut ma = MultiAddr::default();
    for a in r.iter() {
        ma.try_extend(&try_address_to_multiaddr(a).ok()?).ok()?
    }
    Some(ma)
}

/// Try to convert an Ockam Address to a MultiAddr.
pub fn try_address_to_multiaddr(a: &Address) -> Result<MultiAddr, Error> {
    let mut ma = MultiAddr::default();
    match a.transport_type() {
        LOCAL => ma.push_back(Service::new(a.address()))?,
        other => {
            error!(target: "ockam_api", transport = %other, "unsupported transport type");
            return Err(ApiError::message(format!(
                "unknown transport type: {other}"
            )));
        }
    }
    Ok(ma)
}

/// Try to convert an Ockam Address into a MultiAddr.
pub fn addr_to_multiaddr<T: Into<Address>>(a: T) -> Option<MultiAddr> {
    let r: Route = Route::from(a);
    route_to_multiaddr(&r)
}

/// Tells whether the input MultiAddr references a local node or a remote node.
///
/// This should be called before cleaning the MultiAddr.
pub fn is_local_node(ma: &MultiAddr) -> anyhow::Result<bool> {
    let at_rust_node;
    if let Some(p) = ma.iter().next() {
        match p.code() {
            // A MultiAddr starting with "/project" will always reference a remote node.
            Project::CODE => {
                at_rust_node = false;
            }
            // A MultiAddr starting with "/node" will always reference a local node.
            Node::CODE => {
                at_rust_node = true;
            }
            // A "/dnsaddr" will be local if it is "localhost"
            DnsAddr::CODE => {
                at_rust_node = p
                    .cast::<DnsAddr>()
                    .map(|dnsaddr| (*dnsaddr).eq("localhost"))
                    .ok_or_else(|| anyhow!("Invalid \"dnsaddr\" value"))?;
            }
            // A "/ip4" will be local if it matches the loopback address
            Ip4::CODE => {
                at_rust_node = p
                    .cast::<Ip4>()
                    .map(|ip4| ip4.is_loopback())
                    .ok_or_else(|| anyhow!("Invalid \"ip4\" value"))?;
            }
            // A "/ip6" will be local if it matches the loopback address
            Ip6::CODE => {
                at_rust_node = p
                    .cast::<Ip6>()
                    .map(|ip6| ip6.is_loopback())
                    .ok_or_else(|| anyhow!("Invalid \"ip6\" value"))?;
            }
            // A MultiAddr starting with "/service" could reference both local and remote nodes.
            _ => {
                return Err(anyhow!("Invalid address, protocol not supported"));
            }
        }
        Ok(at_rust_node)
    } else {
        Err(anyhow!("Invalid address"))
    }
}

/// Tells whether the input [`Code`] references a local worker.
pub fn local_worker(code: &Code) -> Result<bool> {
    match *code {
        Node::CODE
        | Space::CODE
        | Project::CODE
        | DnsAddr::CODE
        | Ip4::CODE
        | Ip6::CODE
        | Tcp::CODE
        | Secure::CODE => Ok(false),
        Worker::CODE | Service::CODE => Ok(true),

        _ => Err(ApiError::message(format!("unknown transport type: {code}"))),
    }
}

#[cfg(test)]
pub mod test {
    use ockam::identity::SecureChannels;
    use ockam::Result;
    use ockam_core::compat::sync::Arc;
    use ockam_core::flow_control::FlowControls;
    use ockam_core::AsyncTryClone;
    use ockam_identity::IdentityIdentifier;
    use ockam_node::compat::asynchronous::RwLock;
    use ockam_node::Context;
    use ockam_transport_tcp::TcpTransport;

    use crate::cli_state::{traits::*, CliState, IdentityConfig, NodeConfig, VaultConfig};
    use crate::nodes::service::{
        ApiTransport, NodeManagerGeneralOptions, NodeManagerProjectsOptions,
        NodeManagerTransportOptions, NodeManagerTrustOptions,
    };
    use crate::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};

    /// This struct is used by tests, it has two responsibilities:
    /// - guard to delete the cli state at the end of the test, the cli state
    ///   is comprised by some files within the file system, created in a
    ///   temporary directory, and possibly of sub-processes.
    /// - useful access to the NodeManager
    pub struct NodeManagerHandle {
        pub cli_state: CliState,
        pub node_manager: Arc<RwLock<NodeManager>>,
        pub tcp: TcpTransport,
        pub secure_channels: Arc<SecureChannels>,
        pub identifier: IdentityIdentifier,
    }

    impl Drop for NodeManagerHandle {
        fn drop(&mut self) {
            self.cli_state
                .delete(true)
                .expect("cannot delete cli state");
        }
    }

    /// Starts a local node manager and returns a handle to it.
    ///
    /// Be careful: if you drop the returned handle before the end of the test
    /// things *will* break.
    // #[must_use] make sense to enable only on rust 1.67+
    pub async fn start_manager_for_tests(context: &mut Context) -> Result<NodeManagerHandle> {
        let tcp = TcpTransport::create(context).await?;
        let cli_state = CliState::test()?;

        let vault_name = hex::encode(rand::random::<[u8; 4]>());
        let vault = cli_state
            .vaults
            .create_async(&vault_name.clone(), VaultConfig::default())
            .await?
            .get()
            .await?;

        let identity_name = hex::encode(rand::random::<[u8; 4]>());
        let secure_channels = SecureChannels::builder()
            .with_identities_vault(Arc::new(vault))
            .with_identities_repository(cli_state.identities.identities_repository().await?)
            .build();

        let identity = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await
            .unwrap();

        drop(secure_channels);
        let config = IdentityConfig::new(&identity).await;
        cli_state.identities.create(&identity_name, config).unwrap();

        let node_name = hex::encode(rand::random::<[u8; 4]>());
        let node_config = NodeConfig::try_from(&cli_state).unwrap();
        cli_state.nodes.create(&node_name, node_config)?;

        let node_manager = NodeManager::create(
            context,
            NodeManagerGeneralOptions::new(cli_state.clone(), node_name, true, None),
            NodeManagerProjectsOptions::new(Default::default()),
            NodeManagerTransportOptions::new(
                ApiTransport {
                    tt: crate::nodes::models::transport::TransportType::Tcp,
                    tm: crate::nodes::models::transport::TransportMode::Listen,
                    socket_address: "127.0.0.1:123".parse().unwrap(),
                    worker_address: "".into(),
                    processor_address: "".into(),
                    flow_control_id: FlowControls::generate_id(), // FIXME
                },
                tcp.async_try_clone().await?,
            ),
            NodeManagerTrustOptions::new(None),
        )
        .await?;

        let mut node_manager_worker = NodeManagerWorker::new(node_manager);
        let node_manager = node_manager_worker.get().clone();
        let secure_channels = node_manager.read().await.secure_channels.clone();
        context
            .start_worker(
                NODEMANAGER_ADDR,
                node_manager_worker,
                ockam_core::AllowAll,
                ockam_core::AllowAll,
            )
            .await?;

        Ok(NodeManagerHandle {
            cli_state,
            node_manager,
            tcp: tcp.async_try_clone().await?,
            secure_channels: secure_channels.clone(),
            identifier: identity.identifier(),
        })
    }
}
