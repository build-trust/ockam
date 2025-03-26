use crate::authenticator::enrollment_tokens::TokenIssuer;
use crate::authenticator::one_time_code::OneTimeCode;
use crate::cli_state::{ExportedEnrollmentTicket, ProjectRoute};
use crate::control_api::backend::common;
use crate::control_api::backend::common::{
    create_authority_client, parse_identifier, ResourceKind,
};
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, HostPort, NodeName, Project};
use crate::control_api::protocol::ticket::{
    AuthorityInformation, CreateTicketRequest, EnrollProjectRequest, Ticket,
};
use crate::control_api::ControlApiError;
use crate::enroll::enrollment::{EnrollStatus, Enrollment};
use crate::nodes::NodeManager;
use crate::orchestrator::HasSecureClient;
use http::{Method, StatusCode};
use ockam::identity::{Identity, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_ticket(
        &self,
        context: &Context,
        method: Method,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<ControlApiHttpResponse, ControlApiError> {
        let resource_name = ResourceKind::Tickets.name();
        match method {
            Method::POST => {
                if let Some(resource_id) = resource_id {
                    if resource_id == "enroll" {
                        handle_ticket_enroll(context, &self.node_manager, body).await
                    } else {
                        ControlApiHttpResponse::bad_request(
                            &format!("The HTTP path should be /{{node-name}}/{resource_name} or /{{node-name}}/{resource_name}/enroll")
                        )
                    }
                } else {
                    handle_ticket_create(context, &self.node_manager, body).await
                }
            }
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method(method, vec![Method::POST])
            }
        }
    }
}

#[utoipa::path(
    post,
    operation_id = "create_ticket",
    summary = "Create a new Ticket",
    description =
"Create a new Ticket, the main parameters are `attributes`, the list of attributes associated
with the ticket, and `project`, the target project for the ticket. In the vast majority of cases,
specifying the project name will be enough, but it's also possible to specify a custom Ockam
Authority and a custom node acting as a Project.
You can also limit the validity of the ticket by specifying the `usage_count` and `expires_in`
fields.
The ticket is returned as an opaque string that can be used to enroll to a project, either via API
or via CLI with the `ockam project enroll` command.",
    path = "/{node}/tickets",
    tags = ["Tickets"],
    responses(
        (status = CREATED, description = "Successfully created", body = Ticket),
        (status = NOT_FOUND, description = "Specified project not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
    ),
    request_body(
        content = CreateTicketRequest,
        content_type = "application/json",
        description = "Create Ticket request"
    )
)]
async fn handle_ticket_create(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: CreateTicketRequest = common::parse_request_body(body)?;

    let authority_client = create_authority_client(
        node_manager,
        &request.project.to_project_authority().await?,
        &request.identity,
    )
    .await?;

    let result = authority_client
        .create_token(
            context,
            request.attributes.0,
            Some(Duration::from_secs(request.expires_in)),
            Some(request.usage_count),
        )
        .await;

    match result {
        Ok(token) => {
            info!("Successfully created token");
            Ok(ControlApiHttpResponse::with_body(
                StatusCode::CREATED,
                Ticket {
                    encoded: create_encoded_ticket(node_manager, request.project, token)
                        .await?
                        .to_string(),
                },
            )?)
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

    let overridden_project_route;
    let overridden_authority_route;

    let project = match &project_information {
        Project::Existing {
            name: Some(name),
            project_route,
            authority_route,
        } => {
            overridden_project_route = project_route;
            overridden_authority_route = authority_route;
            Some(
                node_manager
                    .cli_state
                    .projects()
                    .get_project_by_name(name)
                    .await?,
            )
        }
        Project::Existing {
            name: None,
            project_route,
            authority_route,
        } => {
            overridden_project_route = project_route;
            overridden_authority_route = authority_route;
            Some(
                node_manager
                    .cli_state
                    .projects()
                    .get_default_project()
                    .await?,
            )
        }
        _ => {
            overridden_project_route = &None;
            overridden_authority_route = &None;
            None
        }
    };
    if let Some(project) = project {
        project_route = if let Some(project_route) = overridden_project_route {
            ProjectRoute::new(project_route.parse()?)?
        } else {
            ProjectRoute::new(project.project_multiaddr().cloned()?)?
        };
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
        authority_route = if let Some(authority_route) = overridden_authority_route {
            authority_route.parse()?
        } else {
            project.authority_multiaddr().cloned()?
        };
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
    operation_id = "project_enroll",
    summary = "Enroll to a Project using a Ticket",
    description = "This API enrolls a node to a Project using the provided Ticket.",
    path = "/{node}/tickets/enroll",
    tags = ["Tickets"],
    responses(
        (status = CREATED, description = "Successfully enrolled", body = AuthorityInformation),
        (status = OK, description = "The node was already enrolled, no change in state", body = AuthorityInformation),
        (status = ACCEPTED, description = "Enrolled, but the project's authority does not match the node's authority.", body = AuthorityInformation),
    ),
    params(
        ("node" = NodeName,),
    ),
    request_body(
        content = EnrollProjectRequest,
        content_type = "application/json",
        description = "Project enrollment request"
    )
)]
async fn handle_ticket_enroll(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: EnrollProjectRequest = common::parse_request_body(body)?;

    let caller_identifier = if let Some(identity) = request.identity {
        parse_identifier(&identity, "identity to enroll")?
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
        HostPort::try_from(address.as_str())?
    } else {
        return Err(ockam_core::Error::new(
            Origin::Api,
            Kind::Internal,
            "Project has no authority address",
        )
        .into());
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
                let different_authority =
                    if let Some(current_authority) = node_manager.project_authority() {
                        current_authority != authority_identifier
                    } else {
                        true
                    };
                if different_authority {
                    // enrolled, but the authority is not being used
                    Ok(ControlApiHttpResponse::with_body(
                        StatusCode::ACCEPTED,
                        authority_info,
                    )?)
                } else {
                    // enrolled, and the authority is already being used
                    Ok(ControlApiHttpResponse::with_body(
                        StatusCode::CREATED,
                        authority_info,
                    )?)
                }
            }
            EnrollStatus::AlreadyEnrolled => {
                // already enrolled
                Ok(ControlApiHttpResponse::with_body(
                    StatusCode::OK,
                    authority_info,
                )?)
            }
            EnrollStatus::UnexpectedStatus(error, status) => {
                Err(ControlApiHttpResponse::with_body(
                    StatusCode::BAD_GATEWAY,
                    ErrorResponse {
                        message: format!("Unexpected status: {} ({})", status, error),
                    },
                )?
                .into())
            }
            EnrollStatus::FailedNoStatus(error) => Err(ControlApiHttpResponse::with_body(
                StatusCode::BAD_GATEWAY,
                ErrorResponse {
                    message: format!("Authority Communication error: {}", error),
                },
            )?
            .into()),
        },
        Err(error) => Err(ControlApiHttpResponse::with_body(
            StatusCode::BAD_GATEWAY,
            ErrorResponse {
                message: format!("Authority Communication error: {}", error),
            },
        )?
        .into()),
    }
}
