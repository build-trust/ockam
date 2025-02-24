use std::sync::Arc;
use std::time::Duration;
use tracing::Level;

use crate::address::get_free_address_for;
use crate::nodes::models::portal::InletStatusView;
use crate::nodes::registry::{InletStateSummary, TcpInletHandle};
use crate::nodes::service::tcp_inlets::session_replacer::selector::OutletMultiAddrSelector;
use crate::nodes::service::tcp_inlets::session_replacer::InletParameters;
use crate::nodes::service::tcp_inlets::terminal_notifier::TcpInletNotifier;
use crate::nodes::service::tcp_inlets::InletSessionReplacer;
use crate::nodes::NodeManager;
use crate::session::replacer::MAX_CONNECT_TIME;
use crate::session::session::{AdditionalSessionOptions, Session};
use ockam::identity::Identifier;
use ockam::Result;
use ockam_abac::{PolicyExpression, Resource, ResourceType};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Route, TryClone};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;
use ockam_transport_core::HostnamePort;

impl NodeManager {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, level = Level::TRACE)]
    pub async fn create_inlet(
        self: &Arc<Self>,
        ctx: &Context,
        listen_address: HostnamePort,
        prefix_route: Route,
        suffix_route: Route,
        target_redundancy: usize,
        outlet_addresses: Vec<MultiAddr>,
        alias: String,
        policy_expression: Option<PolicyExpression>,
        ping_timeout: Option<Duration>,
        wait_for_outlet_duration: Option<Duration>,
        authorized: Option<Identifier>,
        wait_connection: bool,
        secure_channel_identifier: Option<Identifier>,
        enable_udp_puncture: bool,
        // TODO: Introduce mode enum
        disable_tcp_fallback: bool,
        privileged: bool,
        tls_certificate_provider: Option<MultiAddr>,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> Result<InletStatusView> {
        let outlet_addresses_str = outlet_addresses
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        debug! {
            %listen_address,
            prefix = %prefix_route,
            suffix = %suffix_route,
            outlet_addresses = %outlet_addresses_str,
            %alias,
            %enable_udp_puncture,
            %disable_tcp_fallback,
            %skip_handshake,
            %enable_nagle,
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
        let socket_addr = ockam_node::compat::asynchronous::resolve_peer(&listen_address).await?;
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
            if registry.contains_key(&alias) {
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
                .iter()
                .any(|inlet| inlet.bind_address() == listen_addr)
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

        let parameters = Arc::new(InletParameters {
            terminal_notifier: TcpInletNotifier::new(Arc::downgrade(self), alias.clone()),
            outlet_address_selector: OutletMultiAddrSelector::new(outlet_addresses.clone()),
            wait_for_outlet_duration: wait_for_outlet_duration.unwrap_or(MAX_CONNECT_TIME),
            resource: Resource::new(alias.clone(), ResourceType::TcpInlet),
            prefix_route,
            suffix_route,
            authorized,
            policy_expression,
            secure_channel_identifier,
            disable_tcp_fallback,
            tls_certificate_provider,
            skip_handshake,
            enable_nagle,
        });

        let inlet = if privileged {
            #[cfg(privileged_portals_support)]
            {
                let (incoming_access_control, outgoing_access_control) =
                    crate::nodes::service::tcp_inlets::access_control::inlet_access_control(
                        ctx,
                        self,
                        &parameters,
                        None,
                    )
                    .await?;

                // TODO: should options be dependent on the MultiAddr?
                Arc::new(
                    self.tcp_transport
                        .create_privileged_inlet(
                            listen_addr,
                            incoming_access_control,
                            outgoing_access_control,
                        )
                        .await?,
                )
            }
            #[cfg(not(privileged_portals_support))]
            {
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    "Privileged Portals support is not enabled",
                ));
            }
        } else {
            Arc::new(self.tcp_transport.crate_inlet_multi(listen_addr).await?)
        };

        let replacer = InletSessionReplacer {
            node_manager: Arc::downgrade(self),
            udp_transport,
            context: ctx.try_clone()?,
            inlet: inlet.clone(),
            parameters,
            status: None,
        };

        let _ = self
            .cli_state
            .create_tcp_inlet(
                &self.node_name,
                &listen_addr,
                outlet_addresses.clone(),
                &alias,
                privileged,
            )
            .await?;

        let mut sessions = Vec::with_capacity(target_redundancy + 1);
        for _ in 0..target_redundancy {
            // we need replacers with independent status for each session
            let replacer = Arc::new(Mutex::new(replacer.clone_without_status()?));

            let additional_session_options = if enable_udp_puncture {
                Some(AdditionalSessionOptions::create(
                    replacer.clone(),
                    !disable_tcp_fallback,
                    ping_timeout,
                ))
            } else {
                None
            };

            let mut session =
                Session::create(ctx, replacer, additional_session_options, ping_timeout)?;
            session.start_monitoring()?;
            sessions.push(session);
        }

        let mut first_session = {
            let replacer = Arc::new(Mutex::new(replacer));

            let additional_session_options = if enable_udp_puncture {
                Some(AdditionalSessionOptions::create(
                    replacer.clone(),
                    !disable_tcp_fallback,
                    ping_timeout,
                ))
            } else {
                None
            };

            Session::create(ctx, replacer, additional_session_options, ping_timeout)?
        };

        if wait_connection {
            let result = first_session.initial_connect().await;
            if let Err(error) = result {
                warn!(%error, "Failed to connect to the outlet");
            }
        };

        first_session.start_monitoring()?;
        sessions.push(first_session);

        let inlet_info = TcpInletHandle::new(inlet, outlet_addresses.clone(), sessions, privileged);

        // this summary already contains the connection status
        let summary = inlet_info.summary().await;

        self.registry.inlets.insert(alias.clone(), inlet_info);

        let tcp_inlet_status =
            InletStatusView::new(listen_addr, &alias, &outlet_addresses, privileged, summary);

        info! {
            %listen_address,
            outlet_addresses = %outlet_addresses_str,
            %alias,
            "inlet created"
        }

        Ok(tcp_inlet_status)
    }

    pub async fn delete_inlet(&self, context: &Context, alias: &str) -> Result<InletStatusView> {
        info!(%alias, "Handling request to delete inlet portal");
        if let Some(inlet_to_delete) = self.registry.inlets.remove(alias) {
            debug!(%alias, "Successfully removed inlet from node registry");
            if let Err(error) = inlet_to_delete.stop(context).await {
                error!(%alias, %error, "Failed to stop inlet");
            }

            self.resources().delete_resource(&alias.into()).await?;
            self.cli_state
                .delete_tcp_inlet(&self.node_name, alias)
                .await?;
            Ok(InletStatusView::new(
                inlet_to_delete.bind_address(),
                alias,
                &inlet_to_delete.outlet_addresses,
                inlet_to_delete.privileged,
                InletStateSummary::default(),
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

    pub async fn show_inlet(&self, alias: &str) -> Option<InletStatusView> {
        info!(%alias, "Handling request to show inlet portal");
        if let Some(inlet_info) = self.registry.inlets.get(alias) {
            Some(InletStatusView::new(
                inlet_info.bind_address(),
                alias,
                &inlet_info.outlet_addresses,
                inlet_info.privileged,
                inlet_info.summary().await,
            ))
        } else {
            error!(%alias, "Inlet not found in the node registry");
            None
        }
    }

    pub async fn list_inlets(&self) -> Vec<InletStatusView> {
        let mut res = vec![];
        for (alias, info) in self.registry.inlets.entries() {
            res.push(InletStatusView::new(
                info.bind_address(),
                alias,
                &info.outlet_addresses,
                info.privileged,
                info.summary().await,
            ));
        }

        res
    }
}
