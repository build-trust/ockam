use super::{NodeManager, NodeManagerWorker};
use crate::nodes::models::portal::{CreateOutlet, OutletStatusList, TcpOutletInfo};
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::BackgroundNodeClient;
use minicbor::{CborLen, Decode, Encode};
use ockam::tcp::TcpOutletOptions;
use ockam::transport::HostnamePort;
use ockam::{Address, Result};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Request, Response};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, IncomingAccessControl, OutgoingAccessControl};
use ockam_node::Context;
use serde::Serialize;
use std::sync::Arc;

impl NodeManagerWorker {
    #[instrument(skip_all)]
    pub(super) async fn create_outlet(
        &self,
        ctx: &Context,
        create_outlet: CreateOutlet,
    ) -> Result<Response<TcpOutletInfo>, Response<Error>> {
        let CreateOutlet {
            hostname_port,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression,
            tls,
            privileged,
            skip_handshake,
            enable_nagle,
        } = create_outlet;

        let parameters = TcpOutletParameters {
            to: hostname_port,
            tls,
            worker_address: worker_addr,
            policy_expression,
            privileged,
            skip_handshake,
            enable_nagle,
        };

        let parameters = if reachable_from_default_secure_channel {
            parameters.into()
        } else {
            parameters
                .ephemeral()
                .with_reachability(Reachability::DynamicallyConfigured)
        };

        match self.node_manager.create_outlet(ctx, parameters).await {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<TcpOutletInfo>, Response<Error>> {
        match self.node_manager.delete_outlet(worker_addr).await {
            Ok(res) => match res {
                Some(outlet_info) => Ok(Response::ok().body(outlet_info)),
                None => Err(Response::bad_request_no_request(&format!(
                    "Outlet with address {worker_addr} not found"
                ))),
            },
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) fn show_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<TcpOutletInfo>, Response<Error>> {
        match self.node_manager.show_outlet(worker_addr) {
            Some(outlet) => Ok(Response::ok().body(outlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Outlet with address {worker_addr} not found"
            ))),
        }
    }

    pub(crate) async fn get_outlets(&self) -> Result<Response<OutletStatusList>, Response<Error>> {
        let outlets = self.node_manager.list_outlets();
        Ok(Response::ok().body(OutletStatusList(outlets)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TcpOutletParameters {
    #[n(1)] pub to: HostnamePort,
    #[n(2)] pub policy_expression: Option<PolicyExpression>,
    #[n(3)] pub worker_address: Option<Address>,
    #[n(4)] pub tls: bool,
    #[n(5)] pub privileged: bool,
    #[n(6)] pub skip_handshake: bool,
    #[n(7)] pub enable_nagle: bool,
}

impl TcpOutletParameters {
    pub fn new(to: HostnamePort) -> Self {
        Self {
            to,
            policy_expression: None,
            worker_address: None,
            tls: false,
            privileged: false,
            skip_handshake: false,
            enable_nagle: false,
        }
    }

    pub fn with_worker_address(mut self, worker_addr: Address) -> Self {
        self.worker_address = Some(worker_addr);
        self
    }

    pub fn with_policy_expression(mut self, policy_expression: Option<PolicyExpression>) -> Self {
        self.policy_expression = policy_expression;
        self
    }

    pub fn with_tls(mut self, tls: bool) -> Self {
        self.tls = tls;
        self
    }

    pub fn with_privileged(mut self, privileged: bool) -> Self {
        self.privileged = privileged;
        self
    }

    pub fn with_skip_handshake(mut self, skip_handshake: bool) -> Self {
        self.skip_handshake = skip_handshake;
        self
    }

    pub fn with_enable_nagle(mut self, enable_nagle: bool) -> Self {
        self.enable_nagle = enable_nagle;
        self
    }

    pub fn ephemeral(self) -> EphemeralTcpOutletParameters {
        EphemeralTcpOutletParameters {
            parameters: self,
            ephemeral: true,
            custom_access_control: None,
            reachability: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Reachability {
    /// The outlet is reachable via the default secure channel
    #[default]
    ViaDefaultSecureChannel,
    /// Reachability is dynamically configured by a higher level API
    DynamicallyConfigured,
}

pub struct EphemeralTcpOutletParameters {
    pub parameters: TcpOutletParameters,
    pub custom_access_control: Option<(
        Arc<dyn IncomingAccessControl>,
        Arc<dyn OutgoingAccessControl>,
    )>,
    pub reachability: Reachability,
    ephemeral: bool,
}

impl EphemeralTcpOutletParameters {
    pub fn with_reachability(mut self, reachability: Reachability) -> EphemeralTcpOutletParameters {
        self.reachability = reachability;
        self
    }

    pub fn with_custom_access_control(
        mut self,
        incoming_ac: Arc<dyn IncomingAccessControl>,
        outgoing_ac: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        self.custom_access_control = Some((incoming_ac, outgoing_ac));
        self
    }

    /// Returns true if the data structure represents an ephemeral outlet
    pub fn is_ephemeral(&self) -> bool {
        // custom access control cannot be persisted, and when it's populated it's created by
        // a higher level API, a similar consideration applies to reachability
        self.ephemeral
            || self.custom_access_control.is_some()
            || self.reachability != Reachability::ViaDefaultSecureChannel
    }
}

impl From<TcpOutletParameters> for EphemeralTcpOutletParameters {
    fn from(parameters: TcpOutletParameters) -> Self {
        EphemeralTcpOutletParameters {
            parameters,
            custom_access_control: None,
            reachability: Default::default(),
            ephemeral: false,
        }
    }
}

impl NodeManager {
    #[instrument(skip_all)]
    pub async fn create_outlet(
        &self,
        ctx: &Context,
        parameters: impl Into<EphemeralTcpOutletParameters>,
    ) -> Result<TcpOutletInfo> {
        let ephemeral_parameters = parameters.into();
        let is_ephemeral = ephemeral_parameters.is_ephemeral();
        let parameters = ephemeral_parameters.parameters;

        let worker_addr = self
            .registry
            .outlets
            .generate_worker_addr(parameters.worker_address.clone());

        debug!(to = %parameters.to, address = %worker_addr, "creating outlet");

        // Check registry for a duplicated key
        if self.registry.outlets.contains_key(&worker_addr) {
            let message = format!("A TCP outlet with address '{worker_addr}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let (incoming_ac, outgoing_ac) =
            if let Some((incoming_ac, outgoing_ac)) = &ephemeral_parameters.custom_access_control {
                (incoming_ac.clone(), outgoing_ac.clone())
            } else {
                self.access_control(
                    ctx,
                    self.project_authority(),
                    Resource::new(worker_addr.address(), ResourceType::TcpOutlet),
                    Action::HandleMessage,
                    parameters.policy_expression.clone(),
                )
                .await?
            };

        let options = {
            let mut options = TcpOutletOptions::new()
                .with_incoming_access_control(incoming_ac)
                .with_outgoing_access_control(outgoing_ac)
                .with_tls(parameters.tls)
                .set_skip_handshake(parameters.skip_handshake)
                .set_enable_nagle(parameters.enable_nagle);
            if self.project_authority().is_none() {
                for api_transport_flow_control_id in &self.api_transport_flow_control_ids {
                    options = options.as_consumer(api_transport_flow_control_id)
                }
            };
            match ephemeral_parameters.reachability {
                Reachability::ViaDefaultSecureChannel => {
                    // Accept messages from the default secure channel listener
                    if let Some(flow_control_id) =
                        ctx.flow_controls().get_flow_control_with_spawner(
                            &DefaultAddress::SECURE_CHANNEL_LISTENER.into(),
                        )
                    {
                        options = options.as_consumer(&flow_control_id)
                    }
                }
                Reachability::DynamicallyConfigured => {
                    // The reachability is dynamically configured by a higher level API
                }
            }

            options
        };

        let res = if parameters.privileged {
            #[cfg(privileged_portals_support)]
            {
                self.tcp_transport
                    .create_privileged_outlet(worker_addr.clone(), parameters.to.clone(), options)
                    .await
            }
            #[cfg(not(privileged_portals_support))]
            {
                Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    "Privileged Portals support is not enabled",
                ))
            }
        } else {
            self.tcp_transport
                .create_outlet(worker_addr.clone(), parameters.to.clone(), options)
        };

        Ok(match res {
            Ok(_) => {
                let outlet_info = TcpOutletInfo::new(parameters, worker_addr.clone());
                if !is_ephemeral {
                    // the database is the authoritative source, and gets updated first
                    self.cli_state
                        .create_tcp_outlet(&self.node_name, &outlet_info)
                        .await?;
                }

                // TODO: ephemeral outlets should not be persisted in the registry
                //       but InfluxDB would be considered ephemeral, and wouldn't show up
                self.registry
                    .outlets
                    .insert(worker_addr.clone(), outlet_info.clone());

                info!(to = %outlet_info.parameters.to, address = %worker_addr, "outlet created");
                outlet_info
            }
            Err(e) => {
                warn!(at = %parameters.to, err = %e, "Failed to create TCP outlet");
                let message = format!("Failed to create outlet: {}", e);
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    message,
                ));
            }
        })
    }

    pub async fn delete_outlet(&self, worker_addr: &Address) -> Result<Option<TcpOutletInfo>> {
        info!(%worker_addr, "Handling request to delete outlet portal");
        if let Some(outlet_info) = self.registry.outlets.remove(worker_addr) {
            debug!(%worker_addr, "Successfully removed outlet from node registry");

            self.cli_state
                .delete_tcp_outlet(&self.node_name, worker_addr)
                .await?;
            self.resources()
                .delete_resource(&worker_addr.address().into())
                .await?;

            if let Err(e) = self.tcp_transport.stop_outlet(&outlet_info.worker_address) {
                warn!(%worker_addr, %e, "Failed to stop outlet worker");
            }
            trace!(%worker_addr, "Successfully stopped outlet");
            Ok(Some(outlet_info))
        } else {
            warn!(%worker_addr, "Outlet not found in the node registry");
            Ok(None)
        }
    }

    pub fn show_outlet(&self, worker_addr: &Address) -> Option<TcpOutletInfo> {
        info!(%worker_addr, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = self.registry.outlets.get(worker_addr) {
            debug!(%worker_addr, "Outlet not found in node registry");
            Some(outlet_to_show)
        } else {
            error!(%worker_addr, "Outlet not found in the node registry");
            None
        }
    }
}

#[async_trait]
pub trait Outlets {
    #[allow(clippy::too_many_arguments)]
    async fn create_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        privileged: bool,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> miette::Result<TcpOutletInfo>;
}

#[async_trait]
impl Outlets for BackgroundNodeClient {
    #[instrument(skip_all, fields(to = % to, from = ? from))]
    async fn create_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
        privileged: bool,
        skip_handshake: bool,
        enable_nagle: bool,
    ) -> miette::Result<TcpOutletInfo> {
        let mut payload = CreateOutlet::new(
            to,
            tls,
            from.cloned(),
            true,
            privileged,
            skip_handshake,
            enable_nagle,
        );
        if let Some(policy_expression) = policy_expression {
            payload.set_policy_expression(policy_expression);
        }
        let req = Request::post("/node/outlet").body(payload);
        let result: TcpOutletInfo = self.ask(ctx, req).await?;
        Ok(result)
    }
}
