use crate::authenticator::direct::Members;
use crate::control_api::backend::common;
use crate::control_api::backend::common::create_authority_client;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::authority_member::{
    AddOrUpdateAuthorityMemberRequest, AuthorityMember, GetAuthorityMemberRequest,
    ListAuthorityMembersRequest, RemoveAuthorityMemberRequest,
};
use crate::nodes::NodeManager;
use http::StatusCode;
use ockam::identity::Identifier;
use ockam_node::Context;
use std::str::FromStr;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_authority_member(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => {
                    handle_authority_member_add_or_update(context, &self.node_manager, body, id)
                        .await
                }
            },
            "GET" => match resource_id {
                None => handle_authority_member_list(context, &self.node_manager, body).await,
                Some(id) => {
                    handle_authority_member_get(context, &self.node_manager, body, id).await
                }
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => {
                    handle_authority_member_remove(context, &self.node_manager, body, id).await
                }
            },
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "add_or_update_authority_member",
    summary = "Add or update an Authority Member",
    path = "/{node}/authority-member/{member}",
    tags = ["authority-member"],
    responses(
        (status = CREATED, description = "Successfully created"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
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
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: AddOrUpdateAuthorityMemberRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let authority_client =
        match create_authority_client(node_manager, request.authority, &request.identity).await? {
            Ok(authority_client) => authority_client,
            Err(direct_response) => {
                return Ok(direct_response);
            }
        };

    let member_identity = if let Ok(identifier) = Identifier::from_str(member_identity) {
        identifier
    } else {
        return ControlApiHttpResponse::bad_request("Invalid member identity");
    };

    let result = authority_client
        .add_member(context, member_identity, request.attributes)
        .await;
    match result {
        Ok(_) => ControlApiHttpResponse::without_body(StatusCode::CREATED),
        Err(error) => ControlApiHttpResponse::internal_error(error),
    }
}

#[utoipa::path(
    get,
    operation_id = "list_authority_members",
    summary = "List Authority Members",
    path = "/{node}/authority-member",
    tags = ["authority-member"],
    responses(
        (status = OK, description = "Successfully retrieved", body = Vec<AuthorityMember>),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    ),
    request_body(
        content = ListAuthorityMembersRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_authority_member_list(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: ListAuthorityMembersRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let authority_client =
        match create_authority_client(node_manager, request.authority, &request.identity).await? {
            Ok(authority_client) => authority_client,
            Err(direct_response) => {
                return Ok(direct_response);
            }
        };

    let result = authority_client.list_members(context).await;
    match result {
        Ok(members) => {
            let members: Vec<AuthorityMember> = members
                .into_iter()
                .map(|(identity, attributes_entry)| AuthorityMember {
                    identity: identity.to_string(),
                    attributes: attributes_entry.string_attributes(),
                })
                .collect();
            ControlApiHttpResponse::with_body(StatusCode::OK, members)
        }
        Err(error) => ControlApiHttpResponse::internal_error(error),
    }
}

#[utoipa::path(
    get,
    operation_id = "get_authority_member",
    summary = "Get Authority Member",
    path = "/{node}/authority-member/{member}",
    tags = ["authority-member"],
    responses(
        (status = OK, description = "Successfully retrieved", body = AuthorityMember),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("member" = String, description = "Member identity", example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"),
    ),
)]
async fn handle_authority_member_get(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: GetAuthorityMemberRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let authority_client =
        match create_authority_client(node_manager, request.authority, &request.identity).await? {
            Ok(authority_client) => authority_client,
            Err(direct_response) => {
                return Ok(direct_response);
            }
        };

    let member_identity = if let Ok(identifier) = Identifier::from_str(resource_id) {
        identifier
    } else {
        return ControlApiHttpResponse::bad_request("Invalid member identity");
    };

    let result = authority_client
        .show_member(context, &member_identity)
        .await;
    match result {
        Ok(attributes_entry) => ControlApiHttpResponse::with_body(
            StatusCode::OK,
            AuthorityMember {
                identity: member_identity.to_string(),
                attributes: attributes_entry.string_attributes(),
            },
        ),
        Err(error) => {
            //TODO: handle not found
            ControlApiHttpResponse::internal_error(error)
        }
    }
}

#[utoipa::path(
    delete,
    operation_id = "remove_authority_member",
    summary = "Remove an Authority Member",
    path = "/{node}/authority-member/{member}",
    tags = ["authority-member"],
    responses(
        (status = OK, description = "Successfully removed"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("member" = String, description = "Member identity", example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a"),
    ),
    request_body(
        content = RemoveAuthorityMemberRequest,
        content_type = "application/json",
        description = "Remove member request"
    )
)]
async fn handle_authority_member_remove(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: RemoveAuthorityMemberRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let authority_client =
        match create_authority_client(node_manager, request.authority, &request.identity).await? {
            Ok(authority_client) => authority_client,
            Err(direct_response) => {
                return Ok(direct_response);
            }
        };

    let member_identity = if let Ok(identifier) = Identifier::from_str(resource_id) {
        identifier
    } else {
        return ControlApiHttpResponse::bad_request("Invalid member identity");
    };

    let result = authority_client
        .delete_member(context, &member_identity)
        .await;
    match result {
        Ok(_) => ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT),
        Err(error) => {
            //TODO: handle not found
            ControlApiHttpResponse::internal_error(error)
        }
    }
}
