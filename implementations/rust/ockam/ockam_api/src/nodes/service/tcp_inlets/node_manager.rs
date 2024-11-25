use std::sync::Arc;
use std::time::Duration;

use crate::address::get_free_address_for;
use ockam::identity::Identifier;
use ockam::Result;
use ockam_abac::{PolicyExpression, Resource, ResourceType};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{AsyncTryClone, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;

use crate::nodes::models::portal::InletStatus;
use crate::nodes::registry::InletInfo;
use crate::nodes::service::tcp_inlets::InletSessionReplacer;
use crate::nodes::NodeManager;
use crate::session::connection_status::ConnectionStatus;
use crate::session::replacer::{ReplacerOutputKind, SessionReplacer, MAX_CONNECT_TIME};
use crate::session::session::{AdditionalSessionOptions, Session};

impl NodeManager {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_inlet(
        self: &Arc<Self>,
        ctx: &Context,
        listen_address: HostnamePort,
        prefix_route: Route,
        suffix_route: Route,
        outlet_address: MultiAddr,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        // TODO: Introduce mode enum
        disable_tcp_fallback: bool,
        privileged: bool,
        tls_certificate_provider: Option<MultiAddr>,
    ) -> Result<InletStatus> {
        debug! {
            %listen_address,
            prefix = %prefix_route,
            suffix = %suffix_route,
            %outlet_address,
            %alias,
            %enable_udp_puncture,
            %disable_tcp_fallback,
            "creating inlet"
        }

        let udp_transport = if enable_udp_puncture {
            Some(self.udp_transport.clone().ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Transport,
                    Kind::Invalid,
                    "Can't enable UDP puncture or non UDP node",
                )
            })?)
        } else {
            None
        };

        // the port could be zero, to simplify the following code we
        // resolve the address to a full socket address
        let socket_addr =
            ockam_node::compat::asynchronous::resolve_peer(listen_address.to_string()).await?;
        let listen_addr = if listen_address.port() == 0 {
            get_free_address_for(&socket_addr.ip().to_string())
                .map_err(|err| ockam_core::Error::new(Origin::Transport, Kind::Invalid, err))?
        } else {
            socket_addr
        };

        // Check registry for duplicated alias or bind address
        {
            let registry = &self.registry.inlets;

            // Check that there is no entry in the registry with the same alias
            if registry.contains_key(&alias).await {
                let message = format!("A TCP inlet with alias '{alias}' already exists");
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::AlreadyExists,
                    message,
                ));
            }

            // Check that there is no entry in the registry with the same TCP bind address
            if registry
                .values()
                .await
                .iter()
                .any(|inlet| inlet.bind_addr == listen_addr.to_string())
            {
                let message =
                    format!("A TCP inlet with bind tcp address '{listen_addr}' already exists");
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::AlreadyExists,
                    message,
                ));
            }
        }

        let replacer = InletSessionReplacer {
            node_manager: Arc::downgrade(self),
            udp_transport,
            context: ctx.async_try_clone().await?,
            listen_addr: listen_addr.to_string(),
            outlet_addr: outlet_address.clone(),
            prefix_route,
            suffix_route,
            authorized,
            wait_for_outlet_duration: wait_for_outlet_duration.unwrap_or(MAX_CONNECT_TIME),
            resource: Resource::new(alias.clone(), ResourceType::TcpInlet),
            policy_expression,
            secure_channel_identifier,
            disable_tcp_fallback,
            tls_certificate_provider,
            inlet: None,
            connection: None,
            main_route: None,
            additional_secure_channel: None,
            udp_puncture: None,
            additional_route: None,
            privileged,
        };

        let replacer = Arc::new(Mutex::new(replacer));

        let main_replacer: Arc<Mutex<dyn SessionReplacer>> = replacer.clone();

        let _ = self
            .cli_state
            .create_tcp_inlet(
                &self.node_name,
                &listen_addr,
                &outlet_address,
                &alias,
                privileged,
            )
            .await?;

        let additional_session_options = if enable_udp_puncture {
            Some(AdditionalSessionOptions::create(
                replacer.clone(),
                !disable_tcp_fallback,
            ))
        } else {
            None
        };

        let mut session = Session::create(ctx, main_replacer, additional_session_options).await?;

        let outcome = if wait_connection {
            let result = session
                .initial_connect()
                .await
                .map(|outcome| match outcome {
                    ReplacerOutputKind::Inlet(status) => status,
                    _ => {
                        panic!("Unexpected outcome: {:?}", outcome)
                    }
                });

            match result {
                Ok(status) => Some(status),
                Err(err) => {
                    warn!("Failed to create inlet: {err}");
                    None
                }
            }
        } else {
            None
        };

        let connection_status = session.connection_status();

        session.start_monitoring().await?;

        self.registry
            .inlets
            .insert(
                alias.clone(),
                InletInfo::new(
                    &listen_addr.to_string(),
                    outlet_address.clone(),
                    session,
                    privileged,
                ),
            )
            .await;

        let tcp_inlet_status = InletStatus::new(
            listen_addr.to_string(),
            outcome
                .clone()
                .and_then(|s| s.worker.map(|address| address.address().to_string())),
            &alias,
            None,
            outcome.clone().map(|s| s.route.to_string()),
            connection_status,
            outlet_address.to_string(),
            privileged,
        );

        info! {
            %listen_address,
            %outlet_address,
            %alias,
            "inlet created"
        }

        Ok(tcp_inlet_status)
    }

    pub async fn delete_inlet(&self, alias: &str) -> Result<InletStatus> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.registry.inlets.remove(alias).await {
            debug!(%alias, "Successfully removed inlet from node registry");
            inlet_to_delete.session.lock().await.stop().await;
            self.resources().delete_resource(&alias.into()).await?;
            self.cli_state
                .delete_tcp_inlet(&self.node_name, alias)
                .await?;
            Ok(InletStatus::new(
                inlet_to_delete.bind_addr,
                None,
                alias,
                None,
                None,
                ConnectionStatus::Down,
                inlet_to_delete.outlet_addr.to_string(),
                inlet_to_delete.privileged,
            ))
        } else {
            error!(%alias, "Inlet not found in the node registry");
            let message = format!("Inlet with alias {alias} not found");
            Err(ockam_core::Error::new(
                Origin::Node,
                Kind::NotFound,
                message,
            ))
        }
    }

    pub async fn show_inlet(&self, alias: &str) -> Option<InletStatus> {
        info!(%alias, "Handling request to show inlet portal");
        if let Some(inlet_info) = self.registry.inlets.get(alias).await {
            let session = inlet_info.session.lock().await;
            let connection_status = session.connection_status();
            let outcome = session.last_outcome();
            drop(session);
            if let Some(outcome) = outcome {
                if let ReplacerOutputKind::Inlet(status) = outcome {
                    let address = match &status.worker {
                        Some(address) => address.address().to_string(),
                        None => "<>".to_string(),
                    };

                    Some(InletStatus::new(
                        inlet_info.bind_addr.to_string(),
                        address,
                        alias,
                        None,
                        status.route.to_string(),
                        connection_status,
                        inlet_info.outlet_addr.to_string(),
                        inlet_info.privileged,
                    ))
                } else {
                    panic!("Unexpected outcome: {:?}", outcome)
                }
            } else {
                Some(InletStatus::new(
                    inlet_info.bind_addr.to_string(),
                    None,
                    alias,
                    None,
                    None,
                    connection_status,
                    inlet_info.outlet_addr.to_string(),
                    inlet_info.privileged,
                ))
            }
        } else {
            error!(%alias, "Inlet not found in the node registry");
            None
        }
    }

    pub async fn list_inlets(&self) -> Vec<InletStatus> {
        let mut res = vec![];
        for (alias, info) in self.registry.inlets.entries().await {
            let session = info.session.lock().await;
            let connection_status = session.connection_status();
            let outcome = session.last_outcome();
            drop(session);

            let status = if let Some(outcome) = outcome {
                match &outcome {
                    ReplacerOutputKind::Inlet(status) => {
                        let address = match &status.worker {
                            Some(address) => address.address().to_string(),
                            None => "<>".to_string(),
                        };

                        InletStatus::new(
                            &info.bind_addr,
                            address,
                            alias,
                            None,
                            status.route.to_string(),
                            connection_status,
                            info.outlet_addr.to_string(),
                            info.privileged,
                        )
                    }
                    _ => {
                        panic!("Unexpected outcome: {:?}", outcome)
                    }
                }
            } else {
                InletStatus::new(
                    &info.bind_addr,
                    None,
                    alias,
                    None,
                    None,
                    connection_status,
                    info.outlet_addr.to_string(),
                    info.privileged,
                )
            };

            res.push(status);
        }

        res
    }
}
