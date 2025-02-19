use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{Authority, ErrorResponse};
use crate::control_api::ControlApiError;
use crate::nodes::NodeManager;
use crate::orchestrator::project::Project;
use crate::orchestrator::AuthorityNodeClient;
use http::StatusCode;
use ockam::identity::Identifier;
use ockam_core::errcode::{Kind, Origin};
use ockam_multiaddr::MultiAddr;
use serde::de::DeserializeOwned;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

pub(super) enum ResourceKind {
    TcpInlets,
    TcpOutlets,
    Relays,
    Tickets,
    AuthorityMembers,
}

impl ResourceKind {
    pub fn enumerate() -> Vec<Self> {
        vec![
            Self::TcpInlets,
            Self::TcpOutlets,
            Self::Relays,
            Self::Tickets,
            Self::AuthorityMembers,
        ]
    }
    pub fn from_str(resource: &str) -> Option<Self> {
        match resource {
            "tcp-inlets" => Some(Self::TcpInlets),
            "tcp-outlets" => Some(Self::TcpOutlets),
            "relays" => Some(Self::Relays),
            "tickets" => Some(Self::Tickets),
            "authority-members" => Some(Self::AuthorityMembers),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::TcpInlets => "tcp-inlets",
            Self::TcpOutlets => "tcp-outlets",
            Self::Relays => "relays",
            Self::Tickets => "tickets",
            Self::AuthorityMembers => "authority-members",
        }
    }

    pub fn parameter_name(&self) -> &'static str {
        match self {
            Self::TcpInlets => "inlet_name",
            Self::TcpOutlets => "outlet_address",
            Self::Relays => "relay_name",
            Self::Tickets => "",
            Self::AuthorityMembers => "authority_member",
        }
    }
}

impl Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl ControlApiHttpResponse {
    pub(super) fn missing_resource_id<T>(
        resource_kind: ResourceKind,
    ) -> Result<T, ControlApiError> {
        let resource_name = resource_kind.name();
        let resource_name_identifier = resource_kind.parameter_name();

        Err(Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: format!("Missing parameter {resource_name}. The HTTP path should be /{{node-name}}/{resource_name}/{{{resource_name_identifier}}}"),
            },
        )?
            .into())
    }
}

pub async fn create_authority_client(
    node_manager: &Arc<NodeManager>,
    authority: &Authority,
    caller_identifier: &Option<String>,
) -> Result<AuthorityNodeClient, ControlApiError> {
    let caller_identifier = if let Some(identity) = caller_identifier {
        parse_identifier(identity, "selected node identity")?
    } else {
        node_manager.identifier()
    };

    let authority_client: AuthorityNodeClient = match authority {
        Authority::Project { name: Some(name) } => {
            match node_manager
                .cli_state
                .projects()
                .get_project_by_name(name)
                .await
            {
                Ok(project) => {
                    create_project_authority_with_project(
                        node_manager,
                        &project,
                        &caller_identifier,
                    )
                    .await?
                }
                Err(error) => {
                    warn!("Project {name} not found: {error:?}");
                    return ControlApiHttpResponse::not_found("Project not found");
                }
            }
        }
        Authority::Project { name: None } => {
            match node_manager
                .cli_state
                .projects()
                .get_default_project()
                .await
            {
                Ok(project) => {
                    create_project_authority_with_project(
                        node_manager,
                        &project,
                        &caller_identifier,
                    )
                    .await?
                }
                Err(error) => {
                    warn!("No default project: {error:?}");
                    return ControlApiHttpResponse::not_found("Default project not found");
                }
            }
        }

        Authority::Provided { route, identity } => {
            let route = match MultiAddr::try_from(route.as_str()) {
                Ok(route) => route,
                Err(error) => {
                    warn!("Invalid authority route: {error:?}");
                    return ControlApiHttpResponse::bad_request("Invalid authority route");
                }
            };

            let identifier = parse_identifier(identity, "authority identity")?;

            node_manager
                .make_authority_node_client(&identifier, &route, &caller_identifier, None)
                .await?
        }
    };
    Ok(authority_client)
}

pub async fn create_project_authority_with_project(
    node_manager: &Arc<NodeManager>,
    project: &Project,
    caller_identifier: &Identifier,
) -> Result<AuthorityNodeClient, ControlApiError> {
    let is_project_admin = node_manager
        .cli_state
        .is_project_admin(caller_identifier, project)
        .await?;

    let credential_retriever_creator = if is_project_admin {
        node_manager
            .credential_retriever_creators
            .project_admin
            .clone()
    } else {
        None
    };

    let identifier = project.authority_identifier().ok_or_else(|| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Internal,
            "Project has no authority identifier",
        )
    })?;

    Ok(node_manager
        .make_authority_node_client(
            &identifier,
            project.authority_multiaddr()?,
            caller_identifier,
            credential_retriever_creator,
        )
        .await?)
}

pub fn parse_optional_request_body<T: DeserializeOwned + Default>(
    body: Option<Vec<u8>>,
) -> Result<T, ControlApiError> {
    if let Some(body) = body {
        if body.is_empty() {
            Ok(T::default())
        } else {
            match serde_json::from_slice(&body) {
                Ok(request) => Ok(request),
                Err(error) => {
                    warn!("Invalid request body: {error:?}");
                    ControlApiHttpResponse::invalid_body()
                }
            }
        }
    } else {
        Ok(T::default())
    }
}
pub fn parse_request_body<T: DeserializeOwned>(
    body: Option<Vec<u8>>,
) -> Result<T, ControlApiError> {
    let request: T = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(error) => {
                warn!("Invalid request body: {error:?}");
                return ControlApiHttpResponse::invalid_body();
            }
        }
    } else {
        warn!("Missing request body");
        return ControlApiHttpResponse::missing_body();
    };
    Ok(request)
}

pub fn parse_identifier(
    identity: &str,
    identity_parameter_name: &str,
) -> Result<Identifier, ControlApiError> {
    match Identifier::from_str(identity) {
        Ok(identifier) => Ok(identifier),
        Err(error) => {
            warn!("Could not parse the {identity_parameter_name}: {error:?}");
            ControlApiHttpResponse::bad_request(&format!(
                "Could not parse the {identity_parameter_name}. An identity starts with 'I' followed to hexadecimal characters"
            ))
        }
    }
}
