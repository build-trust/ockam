use crate::authenticator::direct::Members;
use crate::control_api::backend::common;
use crate::control_api::backend::common::{create_authority_client, ResourceKind};
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::authority_member::{
    AddOrUpdateAuthorityMemberRequest, AuthorityMember, GetAuthorityMemberRequest,
    ListAuthorityMembersRequest, RemoveAuthorityMemberRequest,
};
use crate::control_api::protocol::common::{Attributes, ErrorResponse, NodeName};
use crate::control_api::ControlApiError;
use crate::nodes::NodeManager;
use http::{Method, StatusCode};
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_authority_member(
        &self,
        method: Method,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<ControlApiHttpResponse, ControlApiError> {
        let context = self.node_manager.ctx();
        match method {
            Method::PUT => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::AuthorityMembers),
                Some(id) => {
                    handle_authority_member_add_or_update(context, &self.node_manager, body, id)
                        .await
                }
            },
            Method::GET => match resource_id {
                None => handle_authority_member_list(context, &self.node_manager, body).await,
                Some(id) => {
                    handle_authority_member_get(context, &self.node_manager, body, id).await
                }
            },
            Method::DELETE => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::AuthorityMembers),
                Some(id) => {
                    handle_authority_member_remove(context, &self.node_manager, body, id).await
                }
            },
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method(
                    method,
                    vec![Method::PUT, Method::GET, Method::DELETE],
                )
            }
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "add_or_update_authority_member",
    summary = "Add or update an Authority Member",
    description =
"Add or update an Authority Member with the specified attributes.
Attributes will overwrite the existing ones if the member already exists.",
    path = "/{node}/authority-members/{member}",
    tags = ["Authority Members"],
    responses(
        (status = CREATED, description = "Successfully created"),
        (status = NOT_FOUND, description = "Specified project not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("member" = String, description = "Member identity", example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"),
    ),
    request_body(
        content = AddOrUpdateAuthorityMemberRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_authority_member_add_or_update(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
    member_identity: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: AddOrUpdateAuthorityMemberRequest = common::parse_request_body(body)?;

    let member_identity =
        common::parse_identifier(member_identity, "Invalid authority member identity")?;
    let authority_client =
        create_authority_client(node_manager, &request.authority, &request.identity).await?;

    let result = authority_client
        .add_member(context, member_identity, request.attributes.0)
        .await;
    match result {
        Ok(_) => Ok(ControlApiHttpResponse::without_body(StatusCode::CREATED)?),
        Err(error) => {
            warn!("Error adding member: {error}");
            ControlApiHttpResponse::internal_error("Adding member failed")
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "list_authority_members",
    summary = "List Authority Members",
    description = "List all members of the Authority.",
    path = "/{node}/authority-members",
    tags = ["Authority Members"],
    responses(
        (status = OK, description = "Successfully retrieved", body = Vec<AuthorityMember>),
        (status = NOT_FOUND, description = "Specified project not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
    ),
    request_body(
        content = ListAuthorityMembersRequest,
        content_type = "application/json",
        description = "Optional list request"
    )
)]
async fn handle_authority_member_list(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: ListAuthorityMembersRequest = common::parse_optional_request_body(body)?;

    let authority_client =
        create_authority_client(node_manager, &request.authority, &request.identity).await?;

    let result = authority_client.list_members(context).await;
    match result {
        Ok(members) => {
            let members: Vec<AuthorityMember> = members
                .into_iter()
                .map(|(identity, attributes_entry)| AuthorityMember {
                    identity: identity.to_string(),
                    attributes: Attributes(attributes_entry.string_attributes()),
                })
                .collect();
            Ok(ControlApiHttpResponse::with_body(StatusCode::OK, members)?)
        }
        Err(error) => {
            warn!("Error listing members: {error}");
            ControlApiHttpResponse::internal_error("Listing members failed")
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "get_authority_member",
    summary = "Get an Authority Member",
    description = "Get an Authority Member given its identity.",
    path = "/{node}/authority-members/{member}",
    tags = ["Authority Members"],
    responses(
        (status = OK, description = "Successfully retrieved", body = AuthorityMember),
        (status = NOT_FOUND, description = "Specified project not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("member" = String, description = "Member identity", example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"),
    ),
    request_body(
        content = GetAuthorityMemberRequest,
        content_type = "application/json",
        description = "Optional get member request"
    )
)]
async fn handle_authority_member_get(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
    member_identity: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: GetAuthorityMemberRequest = common::parse_optional_request_body(body)?;

    let member_identity =
        common::parse_identifier(member_identity, "Invalid authority member identity")?;
    let authority_client =
        create_authority_client(node_manager, &request.authority, &request.identity).await?;

    let result = authority_client
        .show_member(context, &member_identity)
        .await;
    match result {
        Ok(attributes_entry) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::OK,
            AuthorityMember {
                identity: member_identity.to_string(),
                attributes: Attributes(attributes_entry.string_attributes()),
            },
        )?),
        Err(error) => {
            //TODO: handle not found
            warn!("Error getting member: {error}");
            ControlApiHttpResponse::internal_error("Getting member failed")
        }
    }
}

#[utoipa::path(
    delete,
    operation_id = "remove_authority_member",
    summary = "Remove an Authority Member",
    description = "Remove an Authority Member given its identity.",
    path = "/{node}/authority-members/{member}",
    tags = ["Authority Members"],
    responses(
        (status = OK, description = "Successfully removed"),
        (status = NOT_FOUND, description = "Specified project not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("member" = String, description = "Member identity", example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"),
    ),
    request_body(
        content = RemoveAuthorityMemberRequest,
        content_type = "application/json",
        description = "Optional remove member request"
    )
)]
async fn handle_authority_member_remove(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
    member_identity: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: RemoveAuthorityMemberRequest = common::parse_optional_request_body(body)?;

    let member_identity =
        common::parse_identifier(member_identity, "Invalid authority member identity")?;
    let authority_client =
        create_authority_client(node_manager, &request.authority, &request.identity).await?;

    let result = authority_client
        .delete_member(context, &member_identity)
        .await;
    match result {
        Ok(_) => Ok(ControlApiHttpResponse::without_body(
            StatusCode::NO_CONTENT,
        )?),
        Err(error) => {
            //TODO: handle not found
            warn!("Error removing member: {error}");
            ControlApiHttpResponse::internal_error("Deleting member failed")
        }
    }
}
