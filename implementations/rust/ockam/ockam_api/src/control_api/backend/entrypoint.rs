use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
use crate::control_api::protocol::common::ErrorResponse;
use crate::nodes::NodeManager;
use crate::DefaultAddress;
use http::{StatusCode, Uri};
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

        let uri = Uri::from_str(&request.uri).unwrap();
        // The syntax for restful API is:
        // - /NODE_ID/RESOURCE/
        // - /NODE_ID/RESOURCE/RESOURCE_ID/

        let path = uri.path().split('/').collect::<Vec<&str>>();
        if path.len() < 3 {
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

        let resource_kind = path[2].to_lowercase();
        let resource_id = path.get(3).copied();

        let result: ockam_core::Result<ControlApiHttpResponse> = match resource_kind.as_str() {
            "tcp-inlet" => {
                self.handle_tcp_inlet(context, request.method.as_str(), resource_id, request.body)
                    .await
            }
            "tcp-outlet" => {
                self.handle_tcp_outlet(context, request.method.as_str(), resource_id, request.body)
                    .await
            }
            _ => ControlApiHttpResponse::with_body(
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    message: "Unknown resource kind".to_string(),
                },
            ),
        };

        let response = match result {
            Ok(response) => response,
            Err(error) => match error.code().kind {
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
            },
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
