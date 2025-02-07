use crate::authenticator::enrollment_tokens::TokenIssuer;
use crate::cli_state::ExportedEnrollmentTicket;
use crate::control_api::backend::common;
use crate::control_api::backend::common::create_authority_client;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, HostnamePort};
use crate::control_api::protocol::ticket::{
    AuthorityInformation, CreateTicketRequest, EnrollTicketRequest, Ticket,
};
use crate::enroll::enrollment::{EnrollStatus, Enrollment};
use crate::nodes::NodeManager;
use crate::orchestrator::project::Project;
use crate::orchestrator::HasSecureClient;
use http::StatusCode;
use ockam::identity::Identifier;
use ockam_core::errcode::{Kind, Origin};
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_ticket(
        &self,
        context: &Context,
        method: &str,
        _resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => handle_ticket_create(context, &self.node_manager, body).await,
            "POST" => handle_ticket_enroll(context, &self.node_manager, body).await,
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "create_ticket",
    summary = "Create a new Ticket",
    path = "/{node}/ticket",
    tags = ["ticket"],
    responses(
        (status = CREATED, description = "Successfully created", body = Ticket),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    ),
    request_body(
        content = CreateTicketRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_ticket_create(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: CreateTicketRequest = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(_) => return ControlApiHttpResponse::invalid_body(),
        }
    } else {
        return ControlApiHttpResponse::missing_body();
    };

    let authority_client =
        match create_authority_client(node_manager, request.authority, &request.identity).await? {
            Ok(authority_client) => authority_client,
            Err(direct_response) => {
                return Ok(direct_response);
            }
        };

    let result = authority_client
        .create_token(
            context,
            request.attributes,
            Some(Duration::from_secs(request.expires_in)),
            Some(request.usage_count),
        )
        .await;

    match result {
        Ok(token) => ControlApiHttpResponse::with_body(
            StatusCode::CREATED,
            Ticket {
                encoded: String::from(&token),
            },
        ),
        Err(error) => ControlApiHttpResponse::internal_error(error),
    }
}

#[utoipa::path(
    post,
    operation_id = "enroll_ticket",
    summary = "Enroll a Ticket",
    path = "/{node}/ticket",
    tags = ["ticket"],
    responses(
        (status = CREATED, description = "Successfully enrolled, new credential can be used right away", body = AuthorityInformation),
        (status = OK, description = "The node was already enrolled, no change in state", body = AuthorityInformation),
        (status = ACCEPTED, description = "Enrolled, but the node needs a restart", body = AuthorityInformation),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    ),
    request_body(
        content = EnrollTicketRequest,
        content_type = "application/json",
        description = "Enrollment request"
    )
)]
async fn handle_ticket_enroll(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: EnrollTicketRequest = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(_) => return ControlApiHttpResponse::invalid_body(),
        }
    } else {
        return ControlApiHttpResponse::missing_body();
    };

    let caller_identifier = if let Some(identity) = request.identity {
        Identifier::from_str(&identity)?
    } else {
        node_manager.identifier()
    };

    let ticket = if let Ok(ticket) = ExportedEnrollmentTicket::from_str(&request.ticket) {
        ticket.import().await?
    } else {
        return ControlApiHttpResponse::bad_request("Invalid ticket");
    };

    // regardless if the authority is a project or a node, we need to import the project
    let project = ticket.project()?;

    let project: Project = node_manager
        .cli_state
        .projects()
        .import_and_store_project(project.clone())
        .await?;

    let authority_client =
        common::create_project_authority_with_project(node_manager, &project, &caller_identifier)
            .await?;

    let address = if let Some(address) = project.authority_socket_addr() {
        HostnamePort::try_from(address.as_str())?
    } else {
        return Err(ockam_core::Error::new(
            Origin::Api,
            Kind::Internal,
            "Project has no authority address",
        ));
    };

    let authority_route = project.authority_multiaddr().map(|m| m.to_string())?;
    let authority_identifier = project.authority_identifier().ok_or_else(|| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Internal,
            "Project has no authority identifier",
        )
    })?;

    let authority_info = AuthorityInformation {
        route: authority_route,
        identity: authority_identifier.to_string(),
        address,
    };

    let result = authority_client
        .get_secure_client()
        .present_token(context, &ticket.one_time_code)
        .await;
    match result {
        Ok(status) => match status {
            EnrollStatus::EnrolledSuccessfully => {
                let needs_restart =
                    if let Some(current_authority) = node_manager.project_authority() {
                        current_authority != authority_identifier
                    } else {
                        true
                    };
                if needs_restart {
                    // enrolled, but the authority is not being used
                    ControlApiHttpResponse::with_body(StatusCode::ACCEPTED, authority_info)
                } else {
                    // enrolled, and the authority is already being used
                    ControlApiHttpResponse::with_body(StatusCode::CREATED, authority_info)
                }
            }
            EnrollStatus::AlreadyEnrolled => {
                // already enrolled
                ControlApiHttpResponse::with_body(StatusCode::OK, authority_info)
            }
            EnrollStatus::UnexpectedStatus(error, status) => ControlApiHttpResponse::with_body(
                StatusCode::BAD_GATEWAY,
                ErrorResponse {
                    message: format!("Unexpected status: {} ({})", status, error),
                },
            ),
            EnrollStatus::FailedNoStatus(error) => ControlApiHttpResponse::with_body(
                StatusCode::BAD_GATEWAY,
                ErrorResponse {
                    message: format!("Communication error: {}", error),
                },
            ),
        },
        Err(error) => ControlApiHttpResponse::with_body(
            StatusCode::BAD_GATEWAY,
            ErrorResponse {
                message: format!("Communication error: {}", error),
            },
        ),
    }
}
