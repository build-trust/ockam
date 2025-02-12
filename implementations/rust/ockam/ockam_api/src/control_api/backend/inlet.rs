use crate::control_api::backend::common;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::inlet::{CreateInletRequest, InletKind, InletTls};
use crate::control_api::protocol::inlet::{InletStatus, UpdateInletRequest};
use crate::nodes::NodeManager;
use http::StatusCode;
use ockam_abac::{Action, Expr, PolicyExpression, ResourceName};
use ockam_core::compat::rand::random_string;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_inlet(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => handle_tcp_inlet_create(context, &self.node_manager, body).await,
            "GET" => match resource_id {
                None => handle_tcp_inlet_list(&self.node_manager).await,
                Some(id) => handle_tcp_inlet_get(&self.node_manager, id).await,
            },
            "PATCH" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => handle_tcp_inlet_update(&self.node_manager, id, body).await,
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => handle_tcp_inlet_delete(&self.node_manager, id).await,
            },
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method()
            }
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "create_tcp_inlet",
    summary = "Create a new TCP Inlet",
    path = "/{node}/tcp-inlet",
    tags = ["portal", "tcp-inlet"],
    responses(
        (status = CREATED, description = "Successfully created", body = InletStatus),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    ),
    request_body(
        content = CreateInletRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_tcp_inlet_create(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: CreateInletRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    let allow = match request.allow {
        None => None,
        Some(policy) => Some(PolicyExpression::try_from(policy.as_str())?),
    };

    let enable_udp_puncture;
    let disable_tcp_fallback;
    let privileged;

    match request.kind {
        InletKind::Regular => {
            enable_udp_puncture = false;
            disable_tcp_fallback = false;
            privileged = false;
        }
        InletKind::UdpPucture => {
            enable_udp_puncture = true;
            disable_tcp_fallback = false;
            privileged = false;
        }
        InletKind::OnlyUdpPucture => {
            enable_udp_puncture = true;
            disable_tcp_fallback = true;
            privileged = false;
        }
        InletKind::Privileged => {
            enable_udp_puncture = false;
            disable_tcp_fallback = false;
            privileged = true;
        }
        InletKind::PrivilegedUdpPuncture => {
            enable_udp_puncture = true;
            disable_tcp_fallback = false;
            privileged = true;
        }
        InletKind::PrivilegedOnlyUdpPuncture => {
            enable_udp_puncture = true;
            disable_tcp_fallback = true;
            privileged = true;
        }
    }

    let tls_certificate_provider: Option<MultiAddr> = match request.tls {
        InletTls::None => None,
        InletTls::ProjectTls => Some("/project/default/service/tls_certificate_provider".parse()?),
        InletTls::CustomTlsProvider {
            tls_certificate_provider,
        } => Some(tls_certificate_provider.parse()?),
    };

    let authorized = match request.authorized {
        None => None,
        Some(authorized) => Some(authorized.parse()?),
    };

    let result = node_manager
        .create_inlet(
            context,
            request.from.try_into()?,
            Route::default(),
            Route::default(),
            request.to.parse()?,
            request.name.unwrap_or_else(random_string),
            allow,
            None,
            authorized,
            false,
            None,
            enable_udp_puncture,
            disable_tcp_fallback,
            privileged,
            tls_certificate_provider,
            false,
            false,
        )
        .await;
    match result {
        Ok(status) => {
            ControlApiHttpResponse::with_body(StatusCode::CREATED, InletStatus::try_from(status)?)
        }
        Err(error) => {
            // TODO: specialize errors
            // name already exists
            // port already bound
            warn!("Failed to create tcp inlet: {:?}", error);
            ControlApiHttpResponse::internal_error("Failed to create tcp inlet")
        }
    }
}

#[utoipa::path(
    patch,
    operation_id = "update_tcp_inlet",
    summary = "Update a TCP Inlet",
    path = "/{node}/tcp-inlet/{resource_id}",
    tags = ["portal", "tcp-inlet"],
    responses(
        (status = OK, description = "Successfully updated", body = InletStatus),
        (status = NOT_FOUND, description = "Not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
    ),
    request_body(
        content = UpdateInletRequest,
        content_type = "application/json",
        description = "Update request"
    )
)]
async fn handle_tcp_inlet_update(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: UpdateInletRequest = match common::parse_request_body(body) {
        Ok(value) => value,
        Err(value) => return value,
    };

    if node_manager.show_inlet(resource_id).await.is_none() {
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

    handle_tcp_inlet_get(node_manager, resource_id).await
}

#[utoipa::path(
    get,
    operation_id = "list_tcp_inlet",
    summary = "List all TCP Inlets",
    path = "/{node}/tcp-inlet",
    tags = ["portal", "tcp-inlet"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<InletStatus>),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    )
)]
async fn handle_tcp_inlet_list(
    node_manager: &Arc<NodeManager>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let mut inlets: Vec<InletStatus> = Vec::new();

    for status in node_manager.list_inlets().await {
        inlets.push(InletStatus::try_from(status)?);
    }

    ControlApiHttpResponse::with_body(StatusCode::OK, inlets)
}

#[utoipa::path(
    delete,
    operation_id = "delete_tcp_inlet",
    summary = "Delete a TCP Inlet",
    path = "/{node}/tcp-inlet/{resource_id}",
    tags = ["portal", "tcp-inlet"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
    )
)]
async fn handle_tcp_inlet_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let result = node_manager.delete_inlet(resource_id).await;
    match result {
        Ok(_) => ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT),
        Err(error) => {
            warn!("Failed to delete tcp inlet: {:?}", error);
            ControlApiHttpResponse::internal_error("Failed to delete tcp inlet")
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "get_tcp_inlet",
    summary = "Get a TCP Inlet",
    path = "/{node}/tcp-inlet/{resource_id}",
    tags = ["portal", "tcp-inlet"],
    responses(
        (status = OK, description = "Successfully retrieved", body = InletStatus),
        (status = NOT_FOUND, description = "Resource not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
    )
)]
async fn handle_tcp_inlet_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    match node_manager.show_inlet(resource_id).await {
        None => ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND),
        Some(status) => {
            ControlApiHttpResponse::with_body(StatusCode::OK, InletStatus::try_from(status)?)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
    use crate::control_api::protocol::common::{ConnectionStatus, HostnamePort};
    use crate::control_api::protocol::inlet::{CreateInletRequest, InletStatus};
    use crate::test_utils::start_manager_for_tests;
    use crate::DefaultAddress;
    use ockam_core::{Address, NeutralMessage};
    use ockam_node::Context;
    use std::time::Duration;

    #[ockam::test]
    pub async fn tcp_inlet_create_get_list_delete(context: &mut Context) -> ockam_core::Result<()> {
        let handle = start_manager_for_tests(context, None, None).await?;
        let address: Address = DefaultAddress::CONTROL_API.into();

        handle
            .node_manager
            .create_control_api_backend(context, None)?;

        let request = ControlApiHttpRequest {
            method: "PUT".to_string(),
            uri: "/node-name/tcp-inlet".to_string(),
            body: Some(
                serde_json::to_vec(&CreateInletRequest {
                    name: Some("inlet-name".to_string()),
                    kind: Default::default(),
                    tls: Default::default(),
                    from: HostnamePort {
                        hostname: "127.0.0.1".to_string(),
                        port: 0,
                    },
                    to: "/service/outlet".to_string(),
                    identity: None,
                    authorized: None,
                    allow: None,
                    retry_wait: 1000,
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

        // the creation is asynchronous, so the initial status is "down"
        let inlet_status: InletStatus = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(inlet_status.name, "inlet-name");
        assert_eq!(inlet_status.status, ConnectionStatus::Down);
        assert_eq!(inlet_status.current_route, None);
        assert_eq!(inlet_status.to, "/service/outlet");
        assert!(!inlet_status.privileged);
        assert_eq!(inlet_status.bind_address.hostname, "127.0.0.1");
        assert!(inlet_status.bind_address.port > 0);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-inlet/inlet-name".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let inlet_status: InletStatus = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(inlet_status.name, "inlet-name");
        assert_eq!(inlet_status.status, ConnectionStatus::Up);
        assert_eq!(inlet_status.current_route, Some("0#outlet".to_string()));
        assert_eq!(inlet_status.to, "/service/outlet");
        assert!(!inlet_status.privileged);

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-inlet".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let inlets: Vec<InletStatus> = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(inlets.len(), 1);
        assert_eq!(inlets[0].name, "inlet-name");
        assert_eq!(inlets[0].status, ConnectionStatus::Up);
        assert_eq!(inlets[0].current_route, Some("0#outlet".to_string()));
        assert_eq!(inlets[0].to, "/service/outlet");
        assert!(!inlets[0].privileged);

        let request = ControlApiHttpRequest {
            method: "DELETE".to_string(),
            uri: "/node-name/tcp-inlet/inlet-name".to_string(),
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
            uri: "/node-name/tcp-inlet/inlet-name".to_string(),
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
            uri: "/node-name/tcp-inlet".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 200);

        let inlets: Vec<InletStatus> = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(inlets.len(), 0);

        Ok(())
    }
}
