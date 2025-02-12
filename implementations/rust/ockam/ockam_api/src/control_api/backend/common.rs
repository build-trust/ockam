use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::Authority;
use crate::nodes::NodeManager;
use crate::orchestrator::project::Project;
use crate::orchestrator::AuthorityNodeClient;
use ockam::identity::Identifier;
use ockam_core::errcode::{Kind, Origin};
use ockam_multiaddr::MultiAddr;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use std::sync::Arc;

pub async fn create_authority_client(
    node_manager: &Arc<NodeManager>,
    authority: &Authority,
    caller_identifier: &Option<String>,
) -> ockam_core::Result<Result<AuthorityNodeClient, ControlApiHttpResponse>> {
    let caller_identifier = if let Some(identity) = caller_identifier {
        Identifier::from_str(identity)?
    } else {
        node_manager.identifier()
    };

    let authority_client: AuthorityNodeClient = match authority {
        Authority::Project { name: Some(name) } => {
            if let Ok(project) = node_manager
                .cli_state
                .projects()
                .get_project_by_name(name)
                .await
            {
                create_project_authority_with_project(node_manager, &project, &caller_identifier)
                    .await?
            } else {
                warn!("Project {name} not found");
                return Ok(Err(ControlApiHttpResponse::not_found("Project not found")?));
            }
        }
        Authority::Project { name: None } => {
            if let Ok(project) = node_manager
                .cli_state
                .projects()
                .get_default_project()
                .await
            {
                create_project_authority_with_project(node_manager, &project, &caller_identifier)
                    .await?
            } else {
                warn!("No default project");
                return Ok(Err(ControlApiHttpResponse::bad_request(
                    "No default project",
                )?));
            }
        }

        Authority::Provided { route, identity } => {
            let route = if let Ok(route) = MultiAddr::try_from(route.as_str()) {
                route
            } else {
                warn!("Invalid authority route");
                return Ok(Err(ControlApiHttpResponse::bad_request(
                    "Invalid authority route",
                )?));
            };

            let identifier = if let Ok(identifier) = Identifier::from_str(identity) {
                identifier
            } else {
                warn!("Invalid identity");
                return Ok(Err(ControlApiHttpResponse::bad_request(
                    "Invalid identity",
                )?));
            };

            node_manager
                .make_authority_node_client(&identifier, &route, &caller_identifier, None)
                .await?
        }
    };
    Ok(Ok(authority_client))
}

pub async fn create_project_authority_with_project(
    node_manager: &Arc<NodeManager>,
    project: &Project,
    caller_identifier: &Identifier,
) -> ockam_core::Result<AuthorityNodeClient> {
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

    node_manager
        .make_authority_node_client(
            &identifier,
            project.authority_multiaddr()?,
            caller_identifier,
            credential_retriever_creator,
        )
        .await
}

pub fn parse_optional_request_body<T: DeserializeOwned + Default>(
    body: Option<Vec<u8>>,
) -> Result<T, ockam_core::Result<ControlApiHttpResponse>> {
    if let Some(body) = body {
        if body.is_empty() {
            Ok(T::default())
        } else {
            match serde_json::from_slice(&body) {
                Ok(request) => Ok(request),
                Err(error) => {
                    warn!("Invalid request body: {error:?}");
                    Err(ControlApiHttpResponse::invalid_body())
                }
            }
        }
    } else {
        Ok(T::default())
    }
}
pub fn parse_request_body<T: DeserializeOwned>(
    body: Option<Vec<u8>>,
) -> Result<T, ockam_core::Result<ControlApiHttpResponse>> {
    let request: T = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(error) => {
                warn!("Invalid request body: {error:?}");
                return Err(ControlApiHttpResponse::invalid_body());
            }
        }
    } else {
        warn!("Missing request body");
        return Err(ControlApiHttpResponse::missing_body());
    };
    Ok(request)
}
