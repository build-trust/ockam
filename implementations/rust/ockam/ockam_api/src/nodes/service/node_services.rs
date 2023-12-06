use either::Either;

use minicbor::Decoder;

use ockam::identity::{identities, AuthorityService, TrustContext};
use ockam::{Address, Context, Result};
use ockam_abac::Resource;
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_node::WorkerBuilder;

use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::hop::Hop;
use crate::nodes::models::services::{
    ServiceList, ServiceStatus, StartAuthenticatedServiceRequest, StartCredentialsService,
    StartEchoerServiceRequest, StartHopServiceRequest, StartUppercaseServiceRequest,
};
use crate::nodes::registry::CredentialsServiceInfo;
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::NodeManager;
use crate::uppercase::Uppercase;

use super::{actions, NodeManagerWorker};

impl NodeManagerWorker {
    pub(super) async fn start_authenticated_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartAuthenticatedServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_authenticated_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_uppercase_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartUppercaseServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_uppercase_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_echoer_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartEchoerServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_echoer_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_hop_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartHopServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager.start_hop_service_impl(ctx, addr).await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_credentials_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();
        let encoded_identity = body.public_identity();

        let decoded_identity =
            &hex::decode(encoded_identity).map_err(|_| ApiError::core("Unable to decode trust context's public identity when starting credential service."))?;
        let i = identities()
            .await?
            .identities_creation()
            .import(None, decoded_identity)
            .await?;

        let trust_context = TrustContext::new(
            encoded_identity.to_string(),
            Some(AuthorityService::new(
                self.node_manager.identities().credentials(),
                i,
                None,
            )),
        );

        self.node_manager
            .start_credentials_service_impl(ctx, trust_context, addr, oneway)
            .await?;

        Ok(Response::ok(req))
    }

    pub(super) async fn list_services_of_type(
        &self,
        req: &RequestHeader,
        service_type: &str,
    ) -> Result<Response<ServiceList>, Response<Error>> {
        match self.node_manager.list_services_of_type(service_type).await {
            Ok(Either::Left(services)) => Ok(Response::ok(req).body(ServiceList::new(services))),
            Ok(Either::Right(message)) => Err(Response::bad_request(req, &message)),
            Err(e) => Err(Response::internal_error(req, &e.to_string())),
        }
    }

    pub(super) async fn list_services(
        &self,
        req: &RequestHeader,
    ) -> Result<Response<ServiceList>, Response<Error>> {
        match self.node_manager.list_services().await {
            Ok(services) => Ok(Response::ok(req).body(ServiceList::new(services))),
            Err(e) => Err(Response::internal_error(req, &e.to_string())),
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
        let services = self.list_services().await?;
        Ok(Either::Left(
            services
                .into_iter()
                .filter(|service| service.service_type == service_type)
                .collect(),
        ))
    }

    pub async fn list_services(&self) -> Result<Vec<ServiceStatus>> {
        let mut list = Vec::new();
        self.registry
            .authenticated_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::AUTHENTICATED_SERVICE,
                ))
            });
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
            .credentials_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::CREDENTIALS_SERVICE,
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
                        KafkaServiceKind::Consumer => DefaultAddress::KAFKA_CONSUMER,
                        KafkaServiceKind::Producer => DefaultAddress::KAFKA_PRODUCER,
                        KafkaServiceKind::Outlet => DefaultAddress::KAFKA_OUTLET,
                        KafkaServiceKind::Direct => DefaultAddress::KAFKA_DIRECT,
                    },
                ))
            });

        Ok(list)
    }

    pub(super) async fn start_credentials_service_impl(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr).await {
            return Err(ApiError::core("Credentials service exists at this address"));
        }

        self.credentials_service()
            .start(ctx, trust_context, self.identifier(), addr.clone(), !oneway)
            .await?;

        self.registry
            .credentials_services
            .insert(addr.clone(), CredentialsServiceInfo::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_authenticated_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self
            .registry
            .authenticated_services
            .contains_key(&addr)
            .await
        {
            return Err(ApiError::core(
                "Authenticated service exists at this address",
            ));
        }

        let server = Server::new(self.identity_attributes_repository());
        ctx.start_worker(addr.clone(), server).await?;

        self.registry
            .authenticated_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_uppercase_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr).await {
            return Err(ApiError::core("Uppercase service exists at this address"));
        }

        ctx.start_worker(addr.clone(), Uppercase).await?;

        self.registry
            .uppercase_services
            .insert(addr.clone(), Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_echoer_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr).await {
            return Err(ApiError::core("Echoer service exists at this address"));
        }

        let maybe_trust_context_id = self.trust_context.as_ref().map(|c| c.id());
        let resource = Resource::assert_inline(addr.address());
        let ac = self
            .access_control(
                &resource,
                &actions::HANDLE_MESSAGE,
                maybe_trust_context_id,
                None,
            )
            .await?;

        WorkerBuilder::new(Echoer)
            .with_address(addr.clone())
            .with_incoming_access_control_arc(ac)
            .start(ctx)
            .await?;

        self.registry
            .echoer_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_hop_service_impl(&self, ctx: &Context, addr: Address) -> Result<()> {
        if self.registry.hop_services.contains_key(&addr).await {
            return Err(ApiError::core("Hop service exists at this address"));
        }

        ctx.flow_controls()
            .add_consumer(addr.clone(), &self.api_transport_flow_control_id);

        ctx.start_worker(addr.clone(), Hop).await?;

        self.registry
            .hop_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }
}
