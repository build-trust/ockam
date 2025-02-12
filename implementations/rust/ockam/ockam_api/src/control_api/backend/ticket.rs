use crate::authenticator::enrollment_tokens::TokenIssuer;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::cli_state::{ExportedEnrollmentTicket, ProjectRoute};
use crate::control_api::backend::common;
use crate::control_api::backend::common::create_authority_client;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, HostnamePort, Project};
use crate::control_api::protocol::ticket::{
    AuthorityInformation, CreateTicketRequest, EnrollTicketRequest, Ticket,
};
use crate::enroll::enrollment::{EnrollStatus, Enrollment};
use crate::nodes::NodeManager;
use crate::orchestrator::HasSecureClient;
use http::StatusCode;
use ockam::identity::{Identifier, Identity, Vault};
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
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method()
            }
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
    let request: CreateTicketRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let authority_client = match create_authority_client(
        node_manager,
        &request.project.to_project_authority().await?,
        &request.identity,
    )
    .await?
    {
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
        Ok(token) => {
            info!("Successfully created token");
            ControlApiHttpResponse::with_body(
                StatusCode::CREATED,
                Ticket {
                    encoded: create_encoded_ticket(node_manager, request.project, token)
                        .await?
                        .to_string(),
                },
            )
        }
        Err(error) => {
            warn!("Error creating token: {error:?}");
            ControlApiHttpResponse::internal_error("Error creating token")
        }
    }
}

/// Returns a ticket for the given authority
async fn create_encoded_ticket(
    node_manager: &NodeManager,
    project_information: Project,
    one_time_code: OneTimeCode,
) -> ockam_core::Result<ExportedEnrollmentTicket> {
    let project_route;
    let project_identifier;
    let project_name;
    let project_change_history;
    let authority_change_history;
    let authority_route;

    let project = match &project_information {
        Project::Existing { name: Some(name) } => Some(
            node_manager
                .cli_state
                .projects()
                .get_project_by_name(name)
                .await?,
        ),
        Project::Existing { name: None } => Some(
            node_manager
                .cli_state
                .projects()
                .get_default_project()
                .await?,
        ),
        _ => None,
    };
    if let Some(project) = project {
        project_route = ProjectRoute::new(project.project_multiaddr().cloned()?)?;
        project_identifier = if let Some(identifier) = project.project_identifier() {
            identifier
        } else {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Internal,
                "Project has no identifier",
            ));
        };
        project_name = project.name().to_string();
        project_change_history = if let Some(identity) = project.project_identity() {
            identity.change_history().export_as_string()?
        } else {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Internal,
                "Project has no identity",
            ));
        };
        authority_change_history = if let Some(identity) = project.authority_identity() {
            identity.change_history().export_as_string()?
        } else {
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Internal,
                "Project has no authority identity",
            ));
        };
        authority_route = project.authority_multiaddr().cloned()?;
    } else if let Project::Provided {
        project_name: provided_project_name,
        authority_route: provided_authority_route,
        authority_change_history: provided_authority_change_history,
        project_route: provided_project_route,
        project_change_history: provided_project_change_history,
    } = project_information
    {
        let vault = Vault::create_verifying_vault();
        let project_identity =
            Identity::import_from_string(None, &provided_project_change_history, vault).await?;

        authority_change_history = provided_authority_change_history;
        authority_route = provided_authority_route.parse()?;
        project_route = ProjectRoute::new(provided_project_route.parse()?)?;
        project_change_history = provided_project_change_history;
        project_identifier = project_identity.identifier().clone();

        project_name = provided_project_name;
    } else {
        unreachable!();
    }

    Ok(ExportedEnrollmentTicket::new(
        one_time_code,
        project_route,
        project_identifier,
        project_name,
        project_change_history,
        authority_change_history,
        authority_route,
    ))
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
    let request: EnrollTicketRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let caller_identifier = if let Some(identity) = request.identity {
        Identifier::from_str(&identity)?
    } else {
        node_manager.identifier()
    };

    let ticket = match ExportedEnrollmentTicket::from_str(&request.ticket) {
        Ok(ticket) => ticket.import().await?,
        Err(error) => {
            warn!("Error importing ticket: {error:?}");
            return ControlApiHttpResponse::bad_request("Invalid ticket");
        }
    };

    // regardless if the authority is a project or a node, we need to import the project
    let project = ticket.project()?;

    let project: crate::orchestrator::project::Project = node_manager
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
