use crate::control_api::backend::common::ResourceKind;
use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
use crate::control_api::protocol::common::ErrorResponse;
use crate::control_api::ControlApiError;
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use http::{Method, StatusCode, Uri};
use itertools::Itertools;
use ockam_abac::{IncomingAbac, OutgoingAbac, PolicyExpression};
use ockam_core::errcode::Kind;
use ockam_core::{
    async_trait, Address, AllowAll, IncomingAccessControl, NeutralMessage, OutgoingAccessControl,
    Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use std::str::FromStr;
use std::sync::Arc;

pub struct HttpControlNodeApiBackend {
    pub(crate) node_manager: Arc<NodeManager>,
}

impl HttpControlNodeApiBackend {
    pub fn new(node_manager: Arc<NodeManager>) -> Self {
        Self { node_manager }
    }
}

#[async_trait]
impl Worker for HttpControlNodeApiBackend {
    type Message = NeutralMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let request: ControlApiHttpRequest = minicbor::decode(message.payload())?;
        info!(
            "Received Node Control API {} request for {}",
            request.method, request.uri
        );

        let uri = Uri::from_str(&request.uri).unwrap();
        // The syntax for restful API is:
        // - /NODE_ID/RESOURCE/
        // - /NODE_ID/RESOURCE/RESOURCE_ID/

        let path = uri.path().split('/').collect::<Vec<&str>>();
        if path.len() < 3 {
            warn!("Invalid URI: {uri}");
            return context
                .send(
                    message.return_route().clone(),
                    NeutralMessage::from(minicbor::to_vec(&ControlApiHttpResponse::with_body(
                        StatusCode::BAD_REQUEST,
                        ErrorResponse {
                            message: "Invalid URI".to_string(),
                        },
                    )?)?),
                )
                .await;
        }

        // we can ignore the node identifier since it has been already addressed by
        // the reverse proxy
        let _node_identifier = path[1];
        let raw_resource_kind = path[2].to_lowercase();

        let resource_kind = ResourceKind::from_str(&raw_resource_kind);
        let resource_id = path.get(3).copied();
        let method = match Method::try_from(request.method.as_str()) {
            Ok(method) => method,
            Err(_) => {
                warn!("Invalid method: {}", request.method);
                return context
                    .send(
                        message.return_route().clone(),
                        NeutralMessage::from(minicbor::to_vec(
                            &ControlApiHttpResponse::with_body(
                                StatusCode::BAD_REQUEST,
                                ErrorResponse {
                                    message: "Invalid method".to_string(),
                                },
                            )?,
                        )?),
                    )
                    .await;
            }
        };

        let result: Result<ControlApiHttpResponse, ControlApiError> = match resource_kind {
            Some(ResourceKind::TcpInlets) => {
                self.handle_tcp_inlet(context, method, resource_id, request.body)
                    .await
            }
            Some(ResourceKind::TcpOutlets) => {
                self.handle_tcp_outlet(context, method, resource_id, request.body)
                    .await
            }
            Some(ResourceKind::Relays) => {
                self.handle_relay(context, method, resource_id, request.body)
                    .await
            }
            Some(ResourceKind::Tickets) => {
                self.handle_ticket(context, method, resource_id, request.body)
                    .await
            }
            Some(ResourceKind::AuthorityMembers) => {
                self.handle_authority_member(context, method, resource_id, request.body)
                    .await
            }
            None => {
                warn!("Invalid resource kind: {raw_resource_kind}");
                let message = format!(
                    "Invalid resource kind: {raw_resource_kind}. Possible: {}",
                    ResourceKind::enumerate().iter().join(", ")
                );
                ControlApiHttpResponse::bad_request(&message)
            }
        };

        let response = match result {
            Ok(response) => {
                info!("Processed request for {}", request.uri);
                response
            }
            Err(error) => {
                match error {
                    ControlApiError::Response(response) => {
                        warn!(
                            "The API {} {} failed with status {}",
                            request.method,
                            request.uri,
                            StatusCode::try_from(response.status)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| response.status.to_string())
                        );
                        response
                    }
                    ControlApiError::Ockam(error) => {
                        warn!(
                            "The API {} {} failed with an expected error: {error:?}",
                            request.method, request.uri
                        );
                        match error.code().kind {
                            // We make an assumption that every parsing error originates from the
                            // client; This is not necessarily always true, but it's a good approximation
                            Kind::Parse => ControlApiHttpResponse::with_body(
                                StatusCode::BAD_REQUEST,
                                ErrorResponse {
                                    message: error.to_string(),
                                },
                            )?,
                            _ => ControlApiHttpResponse::with_body(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                ErrorResponse {
                                    message: error.to_string(),
                                },
                            )?,
                        }
                    }
                }
            }
        };

        let response = minicbor::to_vec(&response)?;

        context
            .send(
                message.return_route().clone(),
                NeutralMessage::from(response),
            )
            .await
    }
}

impl NodeManager {
    pub fn create_control_api_backend(
        self: &Arc<NodeManager>,
        context: &Context,
        policy_expression: Option<PolicyExpression>,
    ) -> ockam_core::Result<()> {
        let service = HttpControlNodeApiBackend::new(self.clone());
        let address: Address = DefaultAddress::CONTROL_API.into();

        let flow_controls = context.flow_controls();

        let incoming_access_control: Arc<dyn IncomingAccessControl>;
        let outgoing_access_control: Arc<dyn OutgoingAccessControl>;

        match policy_expression {
            Some(policy_expression) => {
                incoming_access_control = Arc::new(IncomingAbac::create(
                    self.secure_channels.identities().identities_attributes(),
                    self.project_authority(),
                    policy_expression.to_expression(),
                ));
                outgoing_access_control = Arc::new(OutgoingAbac::create(
                    context,
                    self.secure_channels.identities().identities_attributes(),
                    self.project_authority(),
                    policy_expression.to_expression(),
                )?);
            }
            None => {
                incoming_access_control = Arc::new(AllowAll);
                outgoing_access_control = Arc::new(AllowAll);
            }
        }

        WorkerBuilder::new(service)
            .with_address(address.clone())
            .with_incoming_access_control_arc(incoming_access_control)
            .with_outgoing_access_control_arc(outgoing_access_control)
            .start(context)?;

        if let Some(secure_channel_listener) = &self.api_sc_listener {
            flow_controls.add_consumer(&address, secure_channel_listener.flow_control_id());
        }

        Ok(())
    }
}
