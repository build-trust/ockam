use miette::miette;
use std::net::{SocketAddrV4, SocketAddrV6};

use ockam::TcpTransport;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Error, Result, Route, TransportType, LOCAL};
use ockam_multiaddr::proto::{
    DnsAddr, Ip4, Ip6, Node, Project, Secure, Service, Space, Tcp, Worker,
};
use ockam_multiaddr::{Code, MultiAddr, Protocol};
use ockam_transport_tcp::{TcpConnection, TcpConnectionOptions, TCP};

use crate::error::ApiError;

/// Try to convert a multi-address to an Ockam route.
pub fn local_multiaddr_to_route(ma: &MultiAddr) -> Result<Route> {
    let mut rb = Route::new();
    for p in ma.iter() {
        match p.code() {
            // Only hops that are directly translated to existing workers are allowed here
            Worker::CODE => {
                let local = p.cast::<Worker>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect worker address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect service address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect secure address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }

            Node::CODE => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    "unexpected code: node. clean_multiaddr should have been called",
                ))
            }

            code @ (Ip4::CODE | Ip6::CODE | DnsAddr::CODE) => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("unexpected code: {code}. The address must be a local address {ma}"),
                ))
            }

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("unsupported protocol {other}"),
                ));
            }
        }
    }

    Ok(rb.into())
}

pub struct MultiAddrToRouteResult {
    pub flow_control_id: Option<FlowControlId>,
    pub route: Route,
    pub tcp_connection: Option<TcpConnection>,
}

pub async fn multiaddr_to_route(
    ma: &MultiAddr,
    tcp: &TcpTransport,
) -> Option<MultiAddrToRouteResult> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();

    let mut flow_control_id = None;
    let mut number_of_tcp_hops = 0;
    let mut tcp_connection = None;

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
                flow_control_id = Some(options.flow_control_id().clone());

                let connection = match tcp.connect(socket_addr.to_string(), options).await {
                    Ok(c) => c,
                    Err(error) => {
                        error!(%error, %socket_addr, "Couldn't connect to Ip4 address");
                        return None;
                    }
                };

                number_of_tcp_hops += 1;
                rb = rb.append(connection.sender_address().clone());

                tcp_connection = Some(connection);
            }
            Ip6::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let ip6 = p.cast::<Ip6>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);

                let options = TcpConnectionOptions::new();
                flow_control_id = Some(options.flow_control_id().clone());

                let connection = match tcp.connect(socket_addr.to_string(), options).await {
                    Ok(c) => c,
                    Err(error) => {
                        error!(%error, %socket_addr, "Couldn't connect to Ip6 address");
                        return None;
                    }
                };

                number_of_tcp_hops += 1;
                rb = rb.append(connection.sender_address().clone());

                tcp_connection = Some(connection);
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
                        flow_control_id = Some(options.flow_control_id().clone());
                        let peer = format!("{}:{}", &*host, *port);

                        let connection = match tcp.connect(&peer, options).await {
                            Ok(c) => c,
                            Err(error) => {
                                error!(%error, %peer, "Couldn't connect to DNS address");
                                return None;
                            }
                        };

                        number_of_tcp_hops += 1;
                        rb = rb.append(connection.sender_address().clone());

                        tcp_connection = Some(connection);

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
        tcp_connection,
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
        .ok_or_else(|| ApiError::core(format!("could not convert {ma} to address")))
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
            return Err(ApiError::core(format!("unknown transport type: {other}")));
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
pub fn is_local_node(ma: &MultiAddr) -> miette::Result<bool> {
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
                    .ok_or_else(|| miette!("Invalid \"dnsaddr\" value"))?;
            }
            // A "/ip4" will be local if it matches the loopback address
            Ip4::CODE => {
                at_rust_node = p
                    .cast::<Ip4>()
                    .map(|ip4| ip4.is_loopback())
                    .ok_or_else(|| miette!("Invalid \"ip4\" value"))?;
            }
            // A "/ip6" will be local if it matches the loopback address
            Ip6::CODE => {
                at_rust_node = p
                    .cast::<Ip6>()
                    .map(|ip6| ip6.is_loopback())
                    .ok_or_else(|| miette!("Invalid \"ip6\" value"))?;
            }
            // A MultiAddr starting with "/service" could reference both local and remote nodes.
            _ => {
                return Err(miette!("Invalid address, protocol not supported"));
            }
        }
        Ok(at_rust_node)
    } else {
        Err(miette!("Invalid address"))
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

        _ => Err(ApiError::core(format!("unknown transport type: {code}"))),
    }
}

#[cfg(test)]
pub mod test_utils {
    use ockam::identity::storage::InMemoryStorage;
    use ockam::identity::utils::AttributesBuilder;
    use ockam::identity::{Identifier, Identity, MAX_CREDENTIAL_VALIDITY};
    use ockam::identity::{SecureChannels, PROJECT_MEMBER_SCHEMA, TRUST_CONTEXT_ID};
    use ockam::Result;
    use ockam_core::compat::sync::Arc;
    use ockam_core::flow_control::FlowControls;
    use ockam_core::AsyncTryClone;

    use ockam_node::Context;
    use ockam_transport_tcp::TcpTransport;

    use crate::cli_state::{
        random_name, traits::*, CliState, IdentityConfig, NodeConfig, VaultConfig,
    };
    use crate::config::cli::{CredentialRetrieverConfig, TrustAuthorityConfig, TrustContextConfig};
    use crate::nodes::service::{
        NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
    };
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
        pub identifier: Identifier,
    }

    impl Drop for NodeManagerHandle {
        fn drop(&mut self) {
            CliState::delete_at(&self.cli_state.dir).expect("cannot delete cli state");
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

        let vault_name = random_name();
        let vault = cli_state
            .vaults
            .create_async(&vault_name.clone(), VaultConfig::default())
            .await?
            .get()
            .await?;

        let identity_name = random_name();

        // Premise: we need an identity and a credential before the node manager starts.
        // Since the LMDB can trigger some race conditions, we first use the memory storage
        // export the identity and credentials,then import in the LMDB after secure-channel
        // has been re-created
        let secure_channels = SecureChannels::builder()
            .with_vault(vault)
            .with_identities_repository(cli_state.identities.identities_repository().await?)
            .with_identities_storage(InMemoryStorage::create())
            .build();

        let identity = create_random_identity(&secure_channels).await?;

        let exported_identity = identity.export()?;

        let attributes = AttributesBuilder::with_schema(PROJECT_MEMBER_SCHEMA)
            .with_attribute(TRUST_CONTEXT_ID.to_vec(), b"test_trust_context_id".to_vec())
            .build();

        let credential = secure_channels
            .identities()
            .credentials()
            .credentials_creation()
            .issue_credential(
                identity.identifier(),
                identity.identifier(),
                attributes,
                MAX_CREDENTIAL_VALIDITY,
            )
            .await
            .unwrap();

        drop(secure_channels);

        let config = IdentityConfig::new(identity.identifier()).await;
        cli_state.identities.create(&identity_name, config).unwrap();

        let node_name = random_name();
        let node_config = NodeConfig::try_from(&cli_state).unwrap();
        cli_state.nodes.create(&node_name, node_config)?;

        let node_manager = InMemoryNode::new(
            context,
            NodeManagerGeneralOptions::new(cli_state.clone(), node_name, None, true, false),
            NodeManagerTransportOptions::new(
                FlowControls::generate_flow_control_id(), // FIXME
                tcp.async_try_clone().await?,
            ),
            NodeManagerTrustOptions::new(Some(TrustContextConfig::new(
                "test_trust_context".to_string(),
                Some(TrustAuthorityConfig::new(
                    hex::encode(&identity.export().unwrap()),
                    Some(CredentialRetrieverConfig::FromMemory(minicbor::to_vec(
                        &credential,
                    )?)),
                )),
            ))),
        )
        .await?;
        let node_manager = Arc::new(node_manager);
        let node_manager_worker = NodeManagerWorker::new(node_manager.clone());
        let secure_channels = node_manager.secure_channels.clone();

        // Import identity, since it doesn't exist in the LMDB storage
        let _ = secure_channels
            .identities()
            .identities_creation()
            .import(Some(identity.identifier()), &exported_identity)
            .await?;

        context
            .start_worker(NODEMANAGER_ADDR, node_manager_worker)
            .await?;

        Ok(NodeManagerHandle {
            cli_state,
            node_manager,
            tcp: tcp.async_try_clone().await?,
            secure_channels: secure_channels.clone(),
            identifier: identity.identifier().clone(),
        })
    }

    async fn create_random_identity(secure_channels: &Arc<SecureChannels>) -> Result<Identity> {
        let identity = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await
            .unwrap();

        Ok(identity)
    }
}
