use either::Either;

use ockam::{Address, Context, Result};
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::api::{Error, Response};
use ockam_node::WorkerBuilder;

use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::hop::Hop;
use crate::nodes::models::node::{NodeResources, NodeStatus};
use crate::nodes::models::services::{
    ServiceStatus, StartEchoerServiceRequest, StartHopServiceRequest, StartUppercaseServiceRequest,
};
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::NodeManager;
use crate::uppercase::Uppercase;

use super::NodeManagerWorker;

impl NodeManagerWorker {
    pub(super) async fn start_uppercase_service(
        &self,
        ctx: &Context,
        request: StartUppercaseServiceRequest,
    ) -> Result<Response, Response<Error>> {
        match self
            .node_manager
            .start_uppercase_service_impl(ctx, request.addr.into())
            .await
        {
            Ok(_) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_echoer_service(
        &self,
        ctx: &Context,
        request: StartEchoerServiceRequest,
    ) -> Result<Response, Response<Error>> {
        match self
            .node_manager
            .start_echoer_service(ctx, request.addr.into())
            .await
        {
            Ok(_) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_hop_service(
        &self,
        ctx: &Context,
        request: StartHopServiceRequest,
    ) -> Result<Response, Response<Error>> {
        match self
            .node_manager
            .start_hop_service(ctx, request.addr.into())
            .await
        {
            Ok(_) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn list_services_of_type(
        &self,
        service_type: &str,
    ) -> Result<Response<Vec<ServiceStatus>>, Response<Error>> {
        match self.node_manager.list_services_of_type(service_type).await {
            Ok(Either::Left(services)) => Ok(Response::ok().body(services)),
            Ok(Either::Right(message)) => Err(Response::bad_request_no_request(&message)),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn list_services(
        &self,
    ) -> Result<Response<Vec<ServiceStatus>>, Response<Error>> {
        Ok(Response::ok().body(self.node_manager.list_services().await))
    }

    #[instrument(skip_all)]
    pub(super) async fn get_node_status(&self) -> Result<Response<NodeStatus>, Response<Error>> {
        match self.node_manager.get_node_status().await {
            Ok(node_status) => Ok(Response::ok().body(node_status)),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    #[instrument(skip_all)]
    pub(super) async fn get_node_resources(
        &self,
    ) -> Result<Response<NodeResources>, Response<Error>> {
        match self.node_manager.get_node_resources().await {
            Ok(node_status) => Ok(Response::ok().body(node_status)),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    pub async fn list_services_of_type(
        &self,
        service_type: &str,
    ) -> Result<Either<Vec<ServiceStatus>, String>> {
        if !DefaultAddress::is_valid(service_type) {
            return Ok(Either::Right(format!(
                "the service {service_type} is not a valid service"
            )));
        };
        let services = self.list_services().await;
        Ok(Either::Left(
            services
                .into_iter()
                .filter(|service| service.service_type == service_type)
                .collect(),
        ))
    }

    pub async fn list_services(&self) -> Vec<ServiceStatus> {
        let mut list = Vec::new();
        self.registry
            .uppercase_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::UPPERCASE_SERVICE,
                ))
            });
        self.registry
            .echoer_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::ECHO_SERVICE,
                ))
            });
        self.registry
            .hop_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::HOP_SERVICE,
                ))
            });
        self.registry
            .kafka_services
            .entries()
            .await
            .iter()
            .for_each(|(address, info)| {
                list.push(ServiceStatus::new(
                    address.address(),
                    match info.kind() {
                        KafkaServiceKind::Inlet => DefaultAddress::KAFKA_INLET,
                        KafkaServiceKind::Outlet => DefaultAddress::KAFKA_OUTLET,
                    },
                ))
            });
        list
    }

    pub(super) async fn start_uppercase_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr).await {
            return Err(ApiError::core(format!(
                "uppercase service already exists at {addr}"
            )));
        }

        ctx.start_worker(addr.clone(), Uppercase).await?;

        info!("uppercase service was initialized at {addr}");

        self.registry
            .uppercase_services
            .insert(addr.clone(), Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_echoer_service(&self, ctx: &Context, addr: Address) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr).await {
            return Err(ApiError::core(format!(
                "echoer service already exists at {addr}"
            )));
        }

        if ctx.is_worker_registered_at(addr.clone()).await? {
            ctx.stop_worker(addr.clone()).await?
        };

        let (incoming_ac, outgoing_ac) = self
            .access_control(
                ctx,
                self.project_authority(),
                Resource::new(addr.address(), ResourceType::Echoer),
                Action::HandleMessage,
                None,
            )
            .await?;

        WorkerBuilder::new(Echoer)
            .with_address(addr.clone())
            .with_incoming_access_control_arc(incoming_ac)
            .with_outgoing_access_control_arc(outgoing_ac)
            .start(ctx)
            .await?;

        info!("echoer service was initialized at {addr}");

        self.registry
            .echoer_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_hop_service(&self, ctx: &Context, addr: Address) -> Result<()> {
        if self.registry.hop_services.contains_key(&addr).await {
            return Err(ApiError::core(format!(
                "hop service already exists at {addr}"
            )));
        }

        for api_transport_flow_control_id in &self.api_transport_flow_control_ids {
            ctx.flow_controls()
                .add_consumer(addr.clone(), api_transport_flow_control_id);
        }

        ctx.start_worker(addr.clone(), Hop).await?;

        info!("hop service was initialized at {addr}");

        self.registry
            .hop_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub async fn get_node_status(&self) -> Result<NodeStatus> {
        let node = self.cli_state.get_node(&self.node_name).await?;
        Ok(NodeStatus::from(&node))
    }

    pub async fn get_node_resources(&self) -> Result<NodeResources> {
        let node = self.cli_state.get_node(&self.node_name).await?;
        let identity = self
            .cli_state
            .get_named_identity_by_identifier(&self.node_identifier)
            .await?;
        let transports = self.get_tcp_listeners();
        let listeners = self.list_secure_channel_listeners().await;
        let inlets = self.list_inlets().await;
        let outlets = self.list_outlets().await;
        let services = self.list_services().await;
        NodeResources::from_parts(
            node,
            identity.name(),
            transports,
            listeners,
            inlets,
            outlets,
            services,
        )
    }
}
