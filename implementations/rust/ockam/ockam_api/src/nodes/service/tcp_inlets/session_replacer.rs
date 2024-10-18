use crate::nodes::service::certificate_provider::ProjectCertificateProvider;
use ockam_transport_tcp::new_certificate_provider_cache;
use std::sync::{Arc, Weak};
use std::time::Duration;

use tokio::time::timeout;

use crate::DefaultAddress;
use ockam::identity::{Identifier, SecureChannel};
use ockam::tcp::TcpInletOptions;
use ockam::udp::{UdpPuncture, UdpPunctureNegotiation, UdpTransport};
use ockam::Result;
use ockam_abac::{Action, PolicyExpression, Resource};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, route, Error, IncomingAccessControl, OutgoingAccessControl, Route};
use ockam_multiaddr::proto::Project as ProjectProto;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpInlet;

use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::service::SecureChannelType;
use crate::nodes::NodeManager;
use crate::session::replacer::{
    AdditionalSessionReplacer, CurrentInletStatus, ReplacerOutcome, ReplacerOutputKind,
    SessionReplacer, MAX_RECOVERY_TIME,
};

pub(super) struct InletSessionReplacer {
    pub(super) node_manager: Weak<NodeManager>,
    pub(super) udp_transport: Option<UdpTransport>,
    pub(super) context: Context,
    pub(super) listen_addr: String,
    pub(super) outlet_addr: MultiAddr,
    pub(super) prefix_route: Route,
    pub(super) suffix_route: Route,
    pub(super) authorized: Option<Identifier>,
    pub(super) wait_for_outlet_duration: Duration,
    pub(super) resource: Resource,
    pub(super) policy_expression: Option<PolicyExpression>,
    pub(super) secure_channel_identifier: Option<Identifier>,
    pub(super) disable_tcp_fallback: bool,
    pub(super) tls_certificate_provider: Option<MultiAddr>,

    // current status
    pub(super) inlet: Option<Arc<TcpInlet>>,
    pub(super) main_route: Option<Route>,

    pub(super) connection: Option<Connection>,

    pub(super) additional_secure_channel: Option<SecureChannel>,
    pub(super) udp_puncture: Option<UdpPuncture>,
    pub(super) additional_route: Option<Route>,
    pub(super) ebpf: bool,
}

impl InletSessionReplacer {
    fn udp_puncture_enabled(&self) -> bool {
        self.udp_transport.is_some()
    }

    async fn access_control(
        &self,
        node_manager: &NodeManager,
    ) -> Result<(
        Arc<dyn IncomingAccessControl>,
        Arc<dyn OutgoingAccessControl>,
    )> {
        let authority = {
            if let Some(p) = self.outlet_addr.first() {
                if let Some(p) = p.cast::<ProjectProto>() {
                    if let Ok(p) = node_manager
                        .cli_state
                        .projects()
                        .get_project_by_name(&p)
                        .await
                    {
                        Some(
                            p.authority_identifier()
                                .ok_or(ApiError::core("no authority identifier"))?,
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        .or(node_manager.project_authority());

        node_manager
            .access_control(
                &self.context,
                authority,
                self.resource.clone(),
                Action::HandleMessage,
                self.policy_expression.clone(),
            )
            .await
    }

    async fn inlet_options(&self, node_manager: &NodeManager) -> Result<TcpInletOptions> {
        let (incoming_ac, outgoing_ac) = self.access_control(node_manager).await?;
        let options = TcpInletOptions::new()
            .with_incoming_access_control(incoming_ac)
            .with_outgoing_access_control(outgoing_ac);

        let options = if self.udp_puncture_enabled() && self.disable_tcp_fallback {
            options.paused()
        } else {
            options
        };

        let options = if let Some(tls_provider) = &self.tls_certificate_provider {
            options.with_tls_certificate_provider(new_certificate_provider_cache(Arc::new(
                ProjectCertificateProvider::new(self.node_manager.clone(), tls_provider.clone()),
            )))
        } else {
            options
        };

        Ok(options)
    }

    async fn create_impl(&mut self, node_manager: &NodeManager) -> Result<ReplacerOutcome> {
        self.pause_inlet().await;
        self.close_connection(node_manager).await;

        let connection = node_manager
            .make_connection(
                &self.context,
                &self.outlet_addr,
                self.secure_channel_identifier
                    .clone()
                    .unwrap_or(node_manager.identifier()),
                self.authorized.clone(),
                Some(self.wait_for_outlet_duration),
            )
            .await?;
        let connection = self.connection.insert(connection);
        let connection_route = connection.route()?;
        let transport_route = connection.transport_route();

        //we expect a fully normalized MultiAddr
        let normalized_route = route![
            self.prefix_route.clone(),
            connection_route.clone(),
            self.suffix_route.clone()
        ];

        // Drop the last address as it will be appended automatically under the hood
        let normalized_stripped_route: Route = normalized_route.clone().modify().pop_back().into();

        // Finally, attempt to create/update inlet using the new route
        let inlet_address = match self.inlet.clone() {
            Some(inlet) => {
                inlet
                    .unpause(&self.context, normalized_stripped_route.clone())
                    .await?;

                inlet.processor_address().cloned()
            }
            None => {
                let options = self.inlet_options(node_manager).await?;
                let inlet = if self.ebpf {
                    #[cfg(ebpf_alias)]
                    {
                        node_manager
                            .tcp_transport
                            .create_raw_inlet(
                                self.listen_addr.clone(),
                                normalized_route.clone(),
                                options,
                            )
                            .await?
                    }
                    #[cfg(not(ebpf_alias))]
                    {
                        return Err(ockam_core::Error::new(
                            Origin::Node,
                            Kind::Internal,
                            "eBPF support is not enabled",
                        ));
                    }
                } else {
                    node_manager
                        .tcp_transport
                        .create_inlet(self.listen_addr.clone(), normalized_route.clone(), options)
                        .await?
                };

                let inlet_address = inlet.processor_address().cloned();

                let inlet = Arc::new(inlet);
                self.inlet = Some(inlet);

                inlet_address
            }
        };

        self.main_route = Some(normalized_stripped_route);

        Ok(ReplacerOutcome {
            ping_route: transport_route,
            kind: ReplacerOutputKind::Inlet(CurrentInletStatus {
                worker: inlet_address,
                route: normalized_route,
            }),
        })
    }

    async fn pause_inlet(&mut self) {
        if let Some(inlet) = self.inlet.as_mut() {
            inlet.pause().await;
        }
    }

    async fn close_inlet(&mut self) {
        if let Some(inlet) = self.inlet.take() {
            // The previous inlet worker needs to be stopped:
            let result = inlet.stop(&self.context).await;

            if let Err(err) = result {
                error!(
                    ?err,
                    "Failed to remove inlet with address {:?}",
                    inlet.processor_address()
                );
            }
        }
    }

    async fn close_connection(&mut self, node_manager: &NodeManager) {
        if let Some(connection) = self.connection.take() {
            let result = connection.close(&self.context, node_manager).await;
            if let Err(err) = result {
                error!(?err, "Failed to close connection");
            }
        }
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

        debug!(%self.outlet_addr, "creating new tcp inlet");

        // The future is given some limited time to succeed.
        // TODO: I believe that every operation inside should have a timeout on its own, the need
        //  of this timeout is questionable (given it's also not adjustable)
        match timeout(MAX_RECOVERY_TIME, self.create_impl(&node_manager)).await {
            Err(_) => {
                warn!(%self.outlet_addr, "timeout creating new tcp inlet");
                Err(ApiError::core("timeout"))
            }
            Ok(Err(e)) => {
                warn!(%self.outlet_addr, err = %e, "error creating new tcp inlet");
                Err(e)
            }
            Ok(Ok(route)) => Ok(route),
        }
    }

    async fn close(&mut self) {
        self.main_route = None;

        let node_manager = if let Some(node_manager) = self.node_manager.upgrade() {
            node_manager
        } else {
            warn!("An inlet close was issued after the NodeManager shut down, skipping.");
            return;
        };

        self.close_inlet().await;
        self.close_connection(&node_manager).await;
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
            .ok_or(Error::new(
                Origin::Node,
                Kind::Invalid,
                "Couldn't create inlet with puncture",
            ))?
            .clone();

        let mut main_route = if let Some(connection) = self.connection.as_ref() {
            connection.route()?
        } else {
            return Err(Error::new(
                Origin::Api,
                Kind::Internal,
                "Error while creating additional session. Connection is absent",
            ));
        };

        let inlet = if let Some(inlet) = self.inlet.clone() {
            inlet
        } else {
            return Err(Error::new(
                Origin::Api,
                Kind::Internal,
                "Error while creating additional session. Inlet is absent",
            ));
        };

        let main_route: Route = main_route.modify().pop_back().into();

        let additional_sc_route =
            route![main_route.clone(), DefaultAddress::SECURE_CHANNEL_LISTENER];

        let additional_sc = node_manager
            .create_secure_channel_internal(
                &self.context,
                additional_sc_route,
                self.secure_channel_identifier
                    .as_ref()
                    .unwrap_or(&node_manager.identifier()),
                self.authorized.clone().map(|authorized| vec![authorized]),
                None,
                // TODO: Have a dedicated timeout
                Some(Duration::from_secs(10)),
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;
        let additional_sc = self.additional_secure_channel.insert(additional_sc);

        let rendezvous_route = route![
            DefaultAddress::get_rendezvous_server_address(),
            DefaultAddress::RENDEZVOUS_SERVICE
        ];

        let puncture = UdpPunctureNegotiation::start_negotiation(
            &self.context,
            route![
                main_route.clone(),
                DefaultAddress::UDP_PUNCTURE_NEGOTIATION_LISTENER
            ],
            &udp_transport,
            rendezvous_route,
            // TODO: Have a dedicated timeout
            Duration::from_secs(10),
        )
        .await?;
        let puncture = self.udp_puncture.insert(puncture);

        // TODO: Have a dedicated timeout duration
        puncture.wait_for_puncture(Duration::from_secs(10)).await?;

        info!("Updating route to UDP");

        additional_sc.update_remote_node_route(route![puncture.sender_address()])?;

        let new_route = route![additional_sc.clone()];
        inlet.unpause(&self.context, new_route.clone()).await?;

        self.additional_route = Some(new_route.clone());

        Ok(new_route)
    }

    async fn close_additional(&mut self, enable_fallback: bool) {
        self.additional_route = None;

        if let Some(inlet) = self.inlet.as_ref() {
            match self.main_route.as_ref() {
                Some(main_route) if enable_fallback => {
                    // Switch Inlet to the main route
                    let res = inlet.unpause(&self.context, main_route.clone()).await;

                    if let Some(err) = res.err() {
                        error!("Error switching Inlet to the main route {}", err);
                    }
                }
                _ => {
                    inlet.pause().await;
                }
            }
        }

        if let Some(secure_channel) = self.additional_secure_channel.take() {
            let res = self.context.stop_worker(secure_channel).await;

            if let Some(err) = res.err() {
                error!("Error closing secure channel {}", err);
            }
        }

        if let Some(puncture) = self.udp_puncture.take() {
            let res = puncture.stop(&self.context).await;

            if let Some(err) = res.err() {
                error!("Error stopping puncture {}", err);
            }
        }
    }
}
