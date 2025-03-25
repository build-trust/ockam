use crate::control_api::backend::common;
use crate::control_api::backend::common::ResourceKind;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, NodeName};
use crate::control_api::protocol::outlet::{
    CreateOutletRequest, CreateOutletRequestValidated, OutletKind, OutletStatus,
    UpdateOutletRequest,
};
use crate::control_api::ControlApiError;
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::NodeManager;
use http::{Method, StatusCode};
use ockam_abac::{Action, Expr, ResourceName};
use ockam_core::errcode::Kind;
use ockam_core::Address;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_outlet(
        &self,
        context: &Context,
        method: Method,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<ControlApiHttpResponse, ControlApiError> {
        match method {
            Method::POST => handle_tcp_outlet_create(context, &self.node_manager, body).await,
            Method::GET => match resource_id {
                None => handle_tcp_outlet_list(&self.node_manager).await,
                Some(id) => handle_tcp_outlet_get(&self.node_manager, id).await,
            },
            Method::PATCH => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::TcpOutlets),
                Some(id) => handle_tcp_outlet_update(&self.node_manager, id, body).await,
            },
            Method::DELETE => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::TcpOutlets),
                Some(id) => handle_tcp_outlet_delete(&self.node_manager, id).await,
            },
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method(
                    method,
                    vec![Method::POST, Method::GET, Method::PATCH, Method::DELETE],
                )
            }
        }
    }
}

#[utoipa::path(
    post,
    operation_id = "create_tcp_outlet",
    summary = "Create a TCP Outlet",
    description =
"The main parameters are the destination `to`, and the service address `name` which is
used to identify the outlet within the node.
The `kind` parameter can be used to create a special outlet, and the `tls` parameter
can be used to connect to TLS endpoints.

The creation will be synchronous, without any blocking operation.",
    path = "/{node}/tcp-outlets",
    tags = ["Portals"],
    responses(
        (status = CREATED, description = "Successfully created", body = OutletStatus),
        (status = CONFLICT, description = "Already exists", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
    ),
    request_body(
        content = CreateOutletRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_tcp_outlet_create(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: CreateOutletRequest = common::parse_request_body(body)?;
    let request = CreateOutletRequestValidated::try_from(request)?;

    let allow = OutletAccessControl::WithPolicyExpression(request.allow.map(|policy| policy));

    let tls = match request.tls {
        None => false,
        Some(_) => true,
    };

    let privileged = match request.kind {
        OutletKind::Regular => false,
        OutletKind::Privileged => true,
    };

    let result = node_manager
        .create_outlet(
            context,
            request.to.try_into()?,
            tls,
            request.name.map(Address::from_string),
            true,
            allow,
            privileged,
            false,
            false,
            false,
        )
        .await;

    match result {
        Ok(outlet_status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::CREATED,
            OutletStatus::from(outlet_status),
        )?),
        Err(error) => match error.code().kind {
            Kind::AlreadyExists => Err(ControlApiHttpResponse::with_body(
                StatusCode::CONFLICT,
                ErrorResponse {
                    message: error.to_string(),
                },
            )?
            .into()),
            _ => ControlApiHttpResponse::internal_error("Failed to create outlet"),
        },
    }
}

#[utoipa::path(
    patch,
    operation_id = "update_tcp_outlet",
    summary = "Update a TCP Outlet",
    description =
"Update a TCP Outlet given its name.

Only the `allow` policy expression can be updated.
To update any other field, delete the TCP Outlet and create a new one.",
    path = "/{node}/tcp-outlets/{name}",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully updated", body = OutletStatus),
        (status = NOT_FOUND, description = "TCP Outlet not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Outlet name (service address)"),
    ),
    request_body(
        content = UpdateOutletRequest,
        content_type = "application/json",
        description = "Update request"
    )
)]
async fn handle_tcp_outlet_update(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
    body: Option<Vec<u8>>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: UpdateOutletRequest = common::parse_request_body(body)?;

    if node_manager.show_outlet(&resource_id.into()).is_none() {
        return ControlApiHttpResponse::not_found("Outlet not found");
    }

    if let Some(allow) = request.allow {
        let expression = match Expr::try_from(allow.as_str()) {
            Ok(allow) => allow,
            Err(error) => {
                warn!("Invalid policy expression: {:?}", error);
                return ControlApiHttpResponse::invalid_body();
            }
        };

        node_manager
            .policies()
            .store_policy_for_resource_name(
                &ResourceName::new(resource_id),
                &Action::HandleMessage,
                &expression,
            )
            .await?;
    }

    handle_tcp_outlet_get(node_manager, resource_id).await
}

#[utoipa::path(
    get,
    operation_id = "list_tcp_outlets",
    summary = "List all TCP Outlets",
    description = "List all TCP Outlets created in a node.",
    path = "/{node}/tcp-outlets",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<OutletStatus>),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    )
)]
async fn handle_tcp_outlet_list(
    node_manager: &Arc<NodeManager>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let outlets: Vec<OutletStatus> = node_manager
        .list_outlets()
        .into_iter()
        .map(OutletStatus::from)
        .collect();
    Ok(ControlApiHttpResponse::with_body(StatusCode::OK, outlets)?)
}

#[utoipa::path(
    get,
    operation_id = "get_tcp_outlet",
    summary = "Get a TCP Outlet",
    description = "Get a TCP Outlet given its name.",
    path = "/{node}/tcp-outlets/{name}",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully retrieved", body = OutletStatus),
        (status = NOT_FOUND, description = "TCP Outlet not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Outlet name (service address)"),
    )
)]
async fn handle_tcp_outlet_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let result = node_manager.show_outlet(&Address::from_string(resource_id));
    match result {
        None => ControlApiHttpResponse::not_found("Outlet not found"),
        Some(status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::OK,
            OutletStatus::from(status),
        )?),
    }
}

#[utoipa::path(
    delete,
    operation_id = "delete_tcp_outlet",
    summary = "Delete a TCP Outlet",
    description = "Delete a TCP Outlet given its name.",
    path = "/{node}/tcp-outlets/{name}",
    tags = ["Portals"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
        (status = NOT_FOUND, description = "TCP Outlet not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Outlet name (service address)"),
    )
)]
async fn handle_tcp_outlet_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    node_manager
        .delete_outlet(&Address::from_string(resource_id))
        .await?;
    Ok(ControlApiHttpResponse::without_body(
        StatusCode::NO_CONTENT,
    )?)
}

#[cfg(test)]
mod test {
    use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
    use crate::control_api::protocol::common::ErrorResponse;
    use crate::control_api::protocol::outlet::{CreateOutletRequest, OutletKind, OutletStatus};
    use crate::test_utils::start_manager_for_tests;
    use crate::DefaultAddress;
    use ockam_core::{Address, NeutralMessage};
    use ockam_node::Context;

    #[ockam::test]
    pub async fn tcp_outlet_create_get_list_delete(
        context: &mut Context,
    ) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;
        let address: Address = DefaultAddress::CONTROL_API.into();

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        let request = ControlApiHttpRequest {
            method: "POST".to_string(),
            uri: "/node-name/tcp-outlets".to_string(),
            body: Some(
                serde_json::to_vec(&CreateOutletRequest {
                    kind: OutletKind::Regular,
                    name: Some("outlet-address".to_string()),
                    to: "127.0.0.1:1234".to_string(),
                    tls: Default::default(),
                    allow: None,
                })
                .unwrap(),
            ),
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 201);

        let outlet_status: OutletStatus = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(outlet_status.name, "outlet-address");
        assert_eq!(outlet_status.to, "127.0.0.1:1234");
        assert!(!outlet_status.privileged);

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlets/outlet-address".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let outlet_status: OutletStatus = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(outlet_status.name, "outlet-address");
        assert_eq!(outlet_status.to, "127.0.0.1:1234");
        assert!(!outlet_status.privileged);

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlets".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let outlets: Vec<OutletStatus> = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(outlets.len(), 1);
        assert_eq!(outlets[0].name, "outlet-address");
        assert_eq!(outlet_status.to, "127.0.0.1:1234");
        assert!(!outlets[0].privileged);

        let request = ControlApiHttpRequest {
            method: "DELETE".to_string(),
            uri: "/node-name/tcp-outlets/outlet-address".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 204);
        assert!(response.body.is_empty());

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlets/outlet-address".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 404);
        let body: ErrorResponse = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(body.message, "Outlet not found");

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlets".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let outlets: Vec<OutletStatus> = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert!(outlets.is_empty());

        Ok(())
    }
}
