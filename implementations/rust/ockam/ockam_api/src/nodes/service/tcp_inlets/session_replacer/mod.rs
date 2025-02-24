use ockam_transport_tcp::new_certificate_provider_cache;
use std::sync::{Arc, Weak};
use std::time::Duration;

use tokio::time::timeout;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::service::certificate_provider::ProjectCertificateProvider;
use crate::nodes::service::tcp_inlets::access_control::inlet_access_control;
use crate::nodes::service::tcp_inlets::terminal_notifier::TcpInletNotifier;
use crate::nodes::service::SecureChannelType;
use crate::nodes::NodeManager;
use crate::session::replacer::{
    ActiveInletRoute, AdditionalSessionReplacer, ReplacerOutcome, ReplacerOutputKind,
    SessionReplacer, MAX_RECOVERY_TIME,
};
use crate::DefaultAddress;
use ockam::identity::{Identifier, SecureChannel};
use ockam::tcp::TcpInletOptions;
use ockam::udp::{UdpPuncture, UdpPunctureNegotiation, UdpTransport};
use ockam::Result;
use ockam_abac::{PolicyExpression, Resource};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, Error, Route, TryClone};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpInlet;
use selector::OutletMultiAddrSelector;

pub(super) mod selector;

pub(super) struct InletParameters {
    pub(super) terminal_notifier: TcpInletNotifier,
    pub(super) outlet_address_selector: OutletMultiAddrSelector,
    pub(super) prefix_route: Route,
    pub(super) suffix_route: Route,
    pub(super) authorized: Option<Identifier>,
    pub(super) wait_for_outlet_duration: Duration,
    pub(super) resource: Resource,
    pub(super) policy_expression: Option<PolicyExpression>,
    pub(super) secure_channel_identifier: Option<Identifier>,
    pub(super) disable_tcp_fallback: bool,
    pub(super) tls_certificate_provider: Option<MultiAddr>,
    pub(super) enable_nagle: bool,
    pub(super) skip_handshake: bool,
}

/// The status of the additional Inlet session.
pub(super) struct AdditionalInletSessionStatus {
    pub(super) secure_channel: SecureChannel,
    pub(super) udp_puncture: UdpPuncture,
}

/// The status of the Inlet session.
pub(super) struct InletSessionStatus {
    pub(super) main_route: Route,
    pub(super) connection: Connection,
    pub(super) last_route_key: String,
    pub(super) additional: Option<AdditionalInletSessionStatus>,
    pub(super) original_multiaddr: MultiAddr,
}

pub(super) struct InletSessionReplacer {
    pub(super) context: Context,
    pub(super) node_manager: Weak<NodeManager>,
    pub(super) inlet: Arc<TcpInlet>,
    pub(super) udp_transport: Option<Arc<UdpTransport>>,
    pub(super) status: Option<InletSessionStatus>,
    pub(super) parameters: Arc<InletParameters>,
}

impl InletSessionReplacer {
    /// Returns a shallow clone instance of the replacer, it shares everything
    /// except the status
    pub(super) fn clone_without_status(&self) -> Result<InletSessionReplacer> {
        Ok(InletSessionReplacer {
            context: self.context.try_clone()?,
            node_manager: self.node_manager.clone(),
            inlet: self.inlet.clone(),
            udp_transport: self.udp_transport.clone(),
            parameters: self.parameters.clone(),
            status: None,
        })
    }

    fn udp_puncture_enabled(&self) -> bool {
        self.udp_transport.is_some()
    }

    async fn inlet_options(
        &self,
        node_manager: &NodeManager,
        original_multi_addr: &MultiAddr,
    ) -> Result<TcpInletOptions> {
        let (incoming_ac, outgoing_ac) = inlet_access_control(
            &self.context,
            node_manager,
            &self.parameters,
            Some(original_multi_addr),
        )
        .await?;

        let options = TcpInletOptions::new()
            .set_skip_handshake(self.parameters.skip_handshake)
            .set_enable_nagle(self.parameters.enable_nagle)
            .with_incoming_access_control(incoming_ac)
            .with_outgoing_access_control(outgoing_ac);

        let options = if self.udp_puncture_enabled() && self.parameters.disable_tcp_fallback {
            // By default, the portal uses the typical route, and switches to UDP puncture once the
            // new route gets established.
            // But when `tcp_fallback` is disabled, we want *all* traffic to pass through *only*
            // the UDP puncture, so we pause the portal until the UDP puncture is complete.
            options.paused()
        } else {
            options
        };

        let options = if let Some(tls_provider) = &self.parameters.tls_certificate_provider {
            options.with_tls_certificate_provider(new_certificate_provider_cache(Arc::new(
                ProjectCertificateProvider::new(self.node_manager.clone(), tls_provider.clone()),
            )))
        } else {
            options
        };

        Ok(options)
    }

    async fn create_impl(&mut self, node_manager: &NodeManager) -> Result<ReplacerOutcome> {
        self.close().await;

        let selected_outlet_addr = self
            .parameters
            .outlet_address_selector
            .select(
                &self.context,
                &self.inlet,
                &node_manager.cli_state.projects(),
            )
            .await?;

        debug!(
            "trying to connect to outlet using {} (derived from {})",
            selected_outlet_addr.selected, selected_outlet_addr.original
        );

        let result = node_manager
            .make_connection(
                &self.context,
                &selected_outlet_addr.selected,
                self.parameters
                    .secure_channel_identifier
                    .clone()
                    .unwrap_or(node_manager.identifier()),
                self.parameters.authorized.clone(),
                Some(self.parameters.wait_for_outlet_duration),
            )
            .await;
        let connection = match result {
            Ok(connection) => connection,
            Err(error) => {
                warn!(original = %selected_outlet_addr.original, selected=%selected_outlet_addr.selected, "failed to instantiate connection: {error:?}");
                return Err(error);
            }
        };

        let connection_route = connection.route()?;
        let transport_route = connection.transport_route();

        //we expect a fully normalized MultiAddr
        let normalized_route = self.parameters.prefix_route.clone()
            + connection_route
            + self.parameters.suffix_route.clone();

        let options = self
            .inlet_options(node_manager, &selected_outlet_addr.original)
            .await?;

        let (original_multiaddr, route_key) =
            selected_outlet_addr.confirm(&self.context, normalized_route.clone(), options)?;

        // Drop the last address as it will be appended automatically under the hood
        let normalized_stripped_route: Route = normalized_route.clone().modify().pop_back().into();

        let inlet_address = self.inlet.processor_address().cloned();

        let main_route = normalized_stripped_route;
        info!(address = ?inlet_address, route = %main_route, "tcp inlet restored");

        // TODO: keep additional connection open?
        self.status = Some(InletSessionStatus {
            main_route,
            connection,
            original_multiaddr,
            last_route_key: route_key,
            additional: None,
        });

        Ok(ReplacerOutcome {
            ping_route: transport_route,
            kind: ReplacerOutputKind::Inlet(ActiveInletRoute {
                route: normalized_route,
            }),
        })
    }
}

#[async_trait]
impl SessionReplacer for InletSessionReplacer {
    async fn create(&mut self) -> Result<ReplacerOutcome> {
        // The addressing scheme is very flexible. Typically, the node connects to
        // the cloud via a secure channel and with another secure channel via
        // relay to the actual outlet on the target node. However, it is also
        // possible that there is just a single secure channel used to go directly
        // to another node.
        let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
            node_manager
        } else {
            return Err(Error::new(
                Origin::Node,
                Kind::Cancelled,
                "Node manager is dropped. Can't create the Inlet.",
            ));
        };

        debug!(%self.parameters.outlet_address_selector, "creating new tcp inlet");

        // The future is given some limited time to succeed.
        // TODO: I believe that every operation inside should have a timeout on its own, the need
        //  of this timeout is questionable (given it's also not adjustable)
        match timeout(MAX_RECOVERY_TIME, self.create_impl(&node_manager)).await {
            Err(_) => {
                warn!(outlet_multiaddresses = %self.parameters.outlet_address_selector, "timeout creating new tcp inlet");
                Err(ApiError::core("timeout"))
            }
            Ok(Err(e)) => {
                warn!(outlet_multiaddresses = %self.parameters.outlet_address_selector, err = %e, "failed to create tcp inlet");
                Err(e)
            }
            Ok(Ok(route)) => Ok(route),
        }
    }

    async fn close(&mut self) {
        if let Some(status) = self.status.take() {
            self.inlet.remove_route(&status.last_route_key);

            let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
                node_manager
            } else {
                warn!("An inlet close was issued after the NodeManager shut down, skipping.");
                return;
            };

            let result = status.connection.close(&self.context, &node_manager);
            if let Err(err) = result {
                error!(?err, "Failed to close connection");
            }
        }
    }

    async fn on_session_down(&self) {
        if let Some(status) = &self.status {
            self.parameters
                .terminal_notifier
                .on_session_down(&status.original_multiaddr)
                .await;
        }
    }

    async fn on_session_replaced(&self) {
        if let Some(status) = &self.status {
            self.parameters
                .terminal_notifier
                .on_session_replaced(&status.original_multiaddr)
                .await;
        }
    }
}

#[async_trait]
impl AdditionalSessionReplacer for InletSessionReplacer {
    async fn create_additional(&mut self) -> Result<Route> {
        let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
            node_manager
        } else {
            return Err(Error::new(
                Origin::Node,
                Kind::Cancelled,
                "Node manager is dropped. Can't start UDP puncture for an Inlet.",
            ));
        };

        let udp_transport = self
            .udp_transport
            .as_ref()
            .ok_or_else(|| {
                Error::new(
                    Origin::Node,
                    Kind::Invalid,
                    "Couldn't create inlet with puncture",
                )
            })?
            .clone();

        let status = if let Some(status) = &mut self.status {
            status
        } else {
            return Err(Error::new(
                Origin::Api,
                Kind::Internal,
                "Error while creating additional session. Connection is absent",
            ));
        };

        let transport_route = status.connection.transport_route();
        // TODO: extract the secure channel listener from status.original_multiaddr
        let additional_sc_route = transport_route.clone() + DefaultAddress::SECURE_CHANNEL_LISTENER;

        let additional_sc = node_manager
            .create_secure_channel_internal(
                &self.context,
                additional_sc_route,
                self.parameters
                    .secure_channel_identifier
                    .as_ref()
                    .unwrap_or(&node_manager.identifier()),
                self.parameters
                    .authorized
                    .clone()
                    .map(|authorized| vec![authorized]),
                None,
                // TODO: Have a dedicated timeout
                Some(Duration::from_secs(10)),
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        let rendezvous_route = route![
            DefaultAddress::get_rendezvous_server_address(),
            DefaultAddress::RENDEZVOUS_SERVICE
        ];

        let mut udp_puncture = UdpPunctureNegotiation::start_negotiation(
            &self.context,
            transport_route + DefaultAddress::UDP_PUNCTURE_NEGOTIATION_LISTENER,
            &udp_transport,
            rendezvous_route,
            // TODO: Have a dedicated timeout
            Duration::from_secs(10),
        )
        .await?;

        // TODO: Have a dedicated timeout duration
        udp_puncture
            .wait_for_puncture(Duration::from_secs(10))
            .await?;

        info!("Updating route to UDP");

        additional_sc.update_remote_node_route(route![udp_puncture.sender_address()])?;

        let additional_route = route![
            additional_sc.clone(),
            status.connection.route()?.recipient()?.clone()
        ];

        status.additional = Some(AdditionalInletSessionStatus {
            secure_channel: additional_sc,
            udp_puncture,
        });

        // drop the mutable borrow
        let status: &InletSessionStatus = self.status.as_ref().unwrap();

        let options = self
            .inlet_options(&node_manager, &status.original_multiaddr)
            .await?;

        self.inlet.update_outlet_route_and_unpause(
            &self.context,
            &status.last_route_key,
            additional_route.clone(),
            options,
        )?;

        Ok(additional_route)
    }

    async fn close_additional(&mut self, enable_fallback: bool) {
        let status = if let Some(status) = &mut self.status {
            status
        } else {
            return;
        };

        let additional = if let Some(additional) = status.additional.take() {
            additional
        } else {
            return;
        };

        if enable_fallback {
            let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
                node_manager
            } else {
                warn!("TCP Inlet fallback to the main route was requested after the NodeManager shut down, skipping.");
                return;
            };

            // turn mutable borrow into immutable
            let status: &InletSessionStatus = self.status.as_ref().unwrap();

            let options = match self
                .inlet_options(&node_manager, &status.original_multiaddr)
                .await
            {
                Ok(options) => options,
                Err(err) => {
                    error!("Error creating TCP Inlet fallback options {}", err);
                    return;
                }
            };

            // Switch Inlet to the main route
            let res = self.inlet.update_outlet_route_and_unpause(
                &self.context,
                &status.last_route_key,
                status.main_route.clone(),
                options,
            );
            if let Some(err) = res.err() {
                error!("Error switching Inlet to the main route {}", err);
            }
        } else {
            // No main_route or no fallback
            self.inlet.pause_route(&status.last_route_key);
        }

        let res = self
            .context
            .stop_address(additional.secure_channel.as_ref());
        if let Some(err) = res.err() {
            error!("Error closing secure channel {}", err);
        }

        let res = additional.udp_puncture.stop(&self.context);
        if let Some(err) = res.err() {
            error!("Error stopping puncture {}", err);
        }
    }
}
