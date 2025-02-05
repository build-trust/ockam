use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::ErrorResponse;
use crate::control_api::protocol::outlet::{
    CreateOutletRequest, OutletKind, OutletStatus, OutletTls, UpdateOutletRequest,
};
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::NodeManager;
use http::StatusCode;
use ockam_abac::{Action, Expr, PolicyExpression, ResourceName};
use ockam_core::errcode::Kind;
use ockam_core::Address;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_outlet(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => handle_tcp_outlet_create(context, &self.node_manager, body).await,
            "GET" => match resource_id {
                None => handle_tcp_outlet_list(&self.node_manager).await,
                Some(id) => handle_tcp_outlet_get(&self.node_manager, id).await,
            },
            "PATCH" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => handle_tcp_outlet_update(&self.node_manager, id, body).await,
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => handle_tcp_outlet_delete(&self.node_manager, id).await,
            },
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "create_tcp_outlet",
    summary = "Create a TCP Outlet",
    path = "/{node}/tcp-outlet",
    tags = ["portal", "tcp-outlet"],
    responses(
        (status = CREATED, description = "Successfully created", body = OutletStatus),
        (status = CONFLICT, description = "Already exists", body = ErrorResponse),
    ),
    params(
        ("node" = String, description = "Destination node name"),
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
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: CreateOutletRequest = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(_error) => {
                warn!("Invalid request body");
                return ControlApiHttpResponse::invalid_body();
            }
        }
    } else {
        warn!("Missing request body");
        return ControlApiHttpResponse::missing_body();
    };

    let allow = OutletAccessControl::WithPolicyExpression(match request.allow {
        None => None,
        Some(policy) => Some(PolicyExpression::try_from(policy.as_str())?),
    });

    let tls = match request.tls {
        OutletTls::None => false,
        OutletTls::Validate => true,
    };

    let priviledged = match request.kind {
        OutletKind::Regular => false,
        OutletKind::Privileged => true,
    };

    let result = node_manager
        .create_outlet(
            context,
            request.to.try_into()?,
            tls,
            request.address.map(Address::from_string),
            true,
            allow,
            priviledged,
        )
        .await;

    match result {
        Ok(outlet_status) => ControlApiHttpResponse::with_body(
            StatusCode::CREATED,
            OutletStatus::from(outlet_status),
        ),
        Err(error) => match error.code().kind {
            Kind::AlreadyExists => ControlApiHttpResponse::with_body(
                StatusCode::CONFLICT,
                ErrorResponse {
                    message: error.to_string(),
                },
            ),
            _ => ControlApiHttpResponse::internal_error(error),
        },
    }
}

#[utoipa::path(
    patch,
    operation_id = "update_tcp_outlet",
    summary = "Update a TCP Outlet",
    path = "/{node}/tcp-outlet/{resource_id}",
    tags = ["portal", "tcp-outlet"],
    responses(
        (status = OK, description = "Successfully updated", body = OutletStatus),
        (status = NOT_FOUND, description = "Not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
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
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: UpdateOutletRequest = if let Some(body) = body {
        match serde_json::from_slice(&body) {
            Ok(request) => request,
            Err(_error) => {
                warn!("Invalid request body");
                return ControlApiHttpResponse::invalid_body();
            }
        }
    } else {
        warn!("Missing request body");
        return ControlApiHttpResponse::missing_body();
    };

    if node_manager.show_outlet(&resource_id.into()).is_none() {
        return ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND);
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
    path = "/{node}/tcp-outlet",
    tags = ["portal", "tcp-outlet"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<OutletStatus>),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    )
)]
async fn handle_tcp_outlet_list(
    node_manager: &Arc<NodeManager>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let outlets: Vec<OutletStatus> = node_manager
        .list_outlets()
        .into_iter()
        .map(OutletStatus::from)
        .collect();
    ControlApiHttpResponse::with_body(StatusCode::OK, outlets)
}

#[utoipa::path(
    get,
    operation_id = "get_tcp_outlet",
    summary = "Get a TCP Outlet",
    path = "/{node}/tcp-outlet/{resource_id}",
    tags = ["portal", "tcp-outlet"],
    responses(
        (status = OK, description = "Successfully retrieved", body = OutletStatus),
        (status = NOT_FOUND, description = "Not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Outlet address"),
    )
)]
async fn handle_tcp_outlet_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let result = node_manager.show_outlet(&Address::from_string(resource_id));
    match result {
        None => ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND),
        Some(status) => {
            ControlApiHttpResponse::with_body(StatusCode::OK, OutletStatus::from(status))
        }
    }
}

#[utoipa::path(
    delete,
    operation_id = "delete_tcp_outlet",
    summary = "Delete a TCP Outlet",
    path = "/{node}/tcp-outlet/{resource_id}",
    tags = ["portal", "tcp-outlet"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
        (status = NOT_FOUND, description = "Not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Outlet address"),
    )
)]
async fn handle_tcp_outlet_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    node_manager
        .delete_outlet(&Address::from_string(resource_id))
        .await?;
    ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod test {
    use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
    use crate::control_api::protocol::common::HostnamePort;
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
            method: "PUT".to_string(),
            uri: "/node-name/tcp-outlet".to_string(),
            body: Some(
                serde_json::to_vec(&CreateOutletRequest {
                    kind: OutletKind::Regular,
                    address: Some("outlet-address".to_string()),
                    to: HostnamePort {
                        hostname: "127.0.0.1".to_string(),
                        port: 1234,
                    },
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
        assert_eq!(outlet_status.address, "outlet-address");
        assert_eq!(outlet_status.to.hostname, "127.0.0.1");
        assert_eq!(outlet_status.to.port, 1234);
        assert!(!outlet_status.privileged);

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlet/outlet-address".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let outlet_status: OutletStatus = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(outlet_status.address, "outlet-address");
        assert_eq!(outlet_status.to.hostname, "127.0.0.1");
        assert_eq!(outlet_status.to.port, 1234);
        assert!(!outlet_status.privileged);

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlet".to_string(),
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
        assert_eq!(outlets[0].address, "outlet-address");
        assert_eq!(outlets[0].to.hostname, "127.0.0.1");
        assert_eq!(outlets[0].to.port, 1234);
        assert!(!outlets[0].privileged);

        let request = ControlApiHttpRequest {
            method: "DELETE".to_string(),
            uri: "/node-name/tcp-outlet/outlet-address".to_string(),
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
            uri: "/node-name/tcp-outlet/outlet-address".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 404);
        assert!(response.body.is_empty());

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-outlet".to_string(),
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
