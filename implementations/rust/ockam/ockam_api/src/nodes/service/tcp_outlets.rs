use ockam::tcp::TcpOutletOptions;
use ockam::transport::HostnamePort;
use ockam::{Address, Result};
use ockam_abac::{Action, PolicyExpression, Resource, ResourceType};
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::async_trait;
use ockam_core::errcode::{Kind, Origin};
use ockam_node::Context;

use crate::nodes::models::portal::{CreateOutlet, OutletAccessControl, OutletStatus};
use crate::nodes::registry::OutletInfo;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::BackgroundNodeClient;

use super::{NodeManager, NodeManagerWorker};

impl NodeManagerWorker {
    #[instrument(skip_all)]
    pub(super) async fn create_outlet(
        &self,
        ctx: &Context,
        create_outlet: CreateOutlet,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        let CreateOutlet {
            hostname_port,
            worker_addr,
            reachable_from_default_secure_channel,
            policy_expression,
            tls,
        } = create_outlet;

        match self
            .node_manager
            .create_outlet(
                ctx,
                hostname_port,
                tls,
                worker_addr,
                reachable_from_default_secure_channel,
                OutletAccessControl::WithPolicyExpression(policy_expression),
            )
            .await
        {
            Ok(outlet_status) => Ok(Response::ok().body(outlet_status)),
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn delete_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.delete_outlet(worker_addr).await {
            Ok(res) => match res {
                Some(outlet_info) => Ok(Response::ok().body(OutletStatus::new(
                    outlet_info.socket_addr,
                    outlet_info.worker_addr.clone(),
                    None,
                ))),
                None => Err(Response::bad_request_no_request(&format!(
                    "Outlet with address {worker_addr} not found"
                ))),
            },
            Err(e) => Err(Response::bad_request_no_request(&format!("{e:?}"))),
        }
    }

    pub(super) async fn show_outlet(
        &self,
        worker_addr: &Address,
    ) -> Result<Response<OutletStatus>, Response<Error>> {
        match self.node_manager.show_outlet(worker_addr).await {
            Some(outlet) => Ok(Response::ok().body(outlet)),
            None => Err(Response::not_found_no_request(&format!(
                "Outlet with address {worker_addr} not found"
            ))),
        }
    }

    pub(super) async fn get_outlets(&self, req: &RequestHeader) -> Response<Vec<OutletStatus>> {
        Response::ok()
            .with_headers(req)
            .body(self.node_manager.list_outlets().await)
    }
}

impl NodeManager {
    #[instrument(skip(self, ctx))]
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn create_outlet(
        &self,
        ctx: &Context,
        hostname_port: HostnamePort,
        tls: bool,
        worker_addr: Option<Address>,
        reachable_from_default_secure_channel: bool,
        access_control: OutletAccessControl,
    ) -> Result<OutletStatus> {
        let worker_addr = self
            .registry
            .outlets
            .generate_worker_addr(worker_addr)
            .await;

        info!(
            "Handling request to create outlet portal at {}:{} with worker {:?}",
            &hostname_port.hostname(),
            hostname_port.port(),
            worker_addr
        );

        // Check registry for a duplicated key
        if self.registry.outlets.contains_key(&worker_addr).await {
            let message = format!("A TCP outlet with address '{worker_addr}' already exists");
            return Err(ockam_core::Error::new(
                Origin::Node,
                Kind::AlreadyExists,
                message,
            ));
        }

        let (incoming_ac, outgoing_ac) = match access_control {
            OutletAccessControl::AccessControl((incoming_ac, outgoing_ac)) => {
                (incoming_ac, outgoing_ac)
            }
            OutletAccessControl::WithPolicyExpression(expression) => {
                self.access_control(
                    ctx,
                    self.project_authority(),
                    Resource::new(worker_addr.address(), ResourceType::TcpOutlet),
                    Action::HandleMessage,
                    expression,
                )
                .await?
            }
        };

        let options = {
            let options = TcpOutletOptions::new()
                .with_incoming_access_control(incoming_ac)
                .with_outgoing_access_control(outgoing_ac)
                .with_tls(tls);
            let options = if self.project_authority().is_none() {
                options.as_consumer(&self.api_transport_flow_control_id)
            } else {
                options
            };
            if reachable_from_default_secure_channel {
                // Accept messages from the default secure channel listener
                if let Some(flow_control_id) = ctx
                    .flow_controls()
                    .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
                {
                    options.as_consumer(&flow_control_id)
                } else {
                    options
                }
            } else {
                options
            }
        };

        let socket_addr = hostname_port.to_socket_addr()?;
        let res = self
            .tcp_transport
            .create_tcp_outlet(worker_addr.clone(), hostname_port, options)
            .await;

        Ok(match res {
            Ok(_) => {
                // TODO: Use better way to store outlets?
                self.registry
                    .outlets
                    .insert(
                        worker_addr.clone(),
                        OutletInfo::new(&socket_addr, Some(&worker_addr)),
                    )
                    .await;

                self.cli_state
                    .create_tcp_outlet(&self.node_name, &socket_addr, &worker_addr, &None)
                    .await?
            }
            Err(e) => {
                warn!(at = %socket_addr, err = %e, "Failed to create TCP outlet");
                let message = format!("Failed to create outlet: {}", e);
                return Err(ockam_core::Error::new(
                    Origin::Node,
                    Kind::Internal,
                    message,
                ));
            }
        })
    }

    pub async fn delete_outlet(&self, worker_addr: &Address) -> Result<Option<OutletInfo>> {
        info!(%worker_addr, "Handling request to delete outlet portal");
        if let Some(deleted_outlet) = self.registry.outlets.remove(worker_addr).await {
            debug!(%worker_addr, "Successfully removed outlet from node registry");

            self.cli_state
                .delete_tcp_outlet(&self.node_name, worker_addr)
                .await?;
            self.resources()
                .delete_resource(&worker_addr.address().into())
                .await?;

            if let Err(e) = self
                .tcp_transport
                .stop_outlet(deleted_outlet.worker_addr.clone())
                .await
            {
                warn!(%worker_addr, %e, "Failed to stop outlet worker");
            }
            trace!(%worker_addr, "Successfully stopped outlet");
            Ok(Some(deleted_outlet))
        } else {
            warn!(%worker_addr, "Outlet not found in the node registry");
            Ok(None)
        }
    }

    pub(super) async fn show_outlet(&self, worker_addr: &Address) -> Option<OutletStatus> {
        info!(%worker_addr, "Handling request to show outlet portal");
        if let Some(outlet_to_show) = self.registry.outlets.get(worker_addr).await {
            debug!(%worker_addr, "Outlet not found in node registry");
            Some(OutletStatus::new(
                outlet_to_show.socket_addr,
                outlet_to_show.worker_addr.clone(),
                None,
            ))
        } else {
            error!(%worker_addr, "Outlet not found in the node registry");
            None
        }
    }
}

#[async_trait]
pub trait Outlets {
    async fn create_outlet(
        &self,
        ctx: &Context,
        to: HostnamePort,
        tls: bool,
        from: Option<&Address>,
        policy_expression: Option<PolicyExpression>,
    ) -> miette::Result<OutletStatus>;
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
    ) -> miette::Result<OutletStatus> {
        let mut payload = CreateOutlet::new(to, tls, from.cloned(), true);
        if let Some(policy_expression) = policy_expression {
            payload.set_policy_expression(policy_expression);
        }
        let req = Request::post("/node/outlet").body(payload);
        let result: OutletStatus = self.ask(ctx, req).await?;
        Ok(result)
    }
}
