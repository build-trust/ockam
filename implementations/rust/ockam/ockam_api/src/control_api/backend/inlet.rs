use crate::control_api::backend::common;
use crate::control_api::backend::common::ResourceKind;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, NodeName};
use crate::control_api::protocol::inlet::{
    CreateInletRequest, CreateInletRequestValidated, InletKind, InletTls,
};
use crate::control_api::protocol::inlet::{InletStatus, UpdateInletRequest};
use crate::control_api::ControlApiError;
use crate::nodes::NodeManager;
use http::{Method, StatusCode};
use ockam_abac::{Action, Expr, ResourceName};
use ockam_core::compat::rand::random_string;
use ockam_core::errcode::Kind;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_inlet(
        &self,
        context: &Context,
        method: Method,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<ControlApiHttpResponse, ControlApiError> {
        match method {
            Method::POST => handle_tcp_inlet_create(context, &self.node_manager, body).await,
            Method::GET => match resource_id {
                None => handle_tcp_inlet_list(&self.node_manager).await,
                Some(id) => handle_tcp_inlet_get(&self.node_manager, id).await,
            },
            Method::PATCH => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::TcpInlets),
                Some(id) => handle_tcp_inlet_update(&self.node_manager, id, body).await,
            },
            Method::DELETE => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::TcpInlets),
                Some(id) => handle_tcp_inlet_delete(&self.node_manager, id).await,
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
    operation_id = "create_tcp_inlet",
    summary = "Create a new TCP Inlet",
    description =
"The main parameters are the destination `to`, and the bind address `from`.
You can also attach a TLS certificate in the `tls` object, restrict access to the TCP Inlet with
`authorized` and `allow`, or select a specialized Portal with `kind`.

The creation will be asynchronous and the initial status will be `down`.",
    path = "/{node}/tcp-inlets",
    tags = ["Portals"],
    responses(
        (status = CREATED, description = "Successfully created", body = InletStatus),
        (status = CONFLICT, description = "TCP Inlet with the same name or port already exists", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
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
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: CreateInletRequest = common::parse_request_body(body)?;
    let request = CreateInletRequestValidated::try_from(request)?;

    let enable_udp_puncture;
    let disable_tcp_fallback;
    let privileged;

    match request.kind {
        InletKind::Regular => {
            enable_udp_puncture = false;
            disable_tcp_fallback = false;
            privileged = false;
        }
        InletKind::UdpPuncture => {
            enable_udp_puncture = true;
            disable_tcp_fallback = false;
            privileged = false;
        }
        InletKind::OnlyUdpPuncture => {
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
        None => None,
        Some(tls) => match tls {
            InletTls::ProjectTls => {
                let default_project = match node_manager
                    .cli_state
                    .projects()
                    .get_default_project()
                    .await
                {
                    Ok(project) => project,
                    Err(error) => {
                        warn!("Failed to get default project: {:?}", error);
                        return ControlApiHttpResponse::internal_error(
                            "Failed to get default project",
                        );
                    }
                };
                let default_project_name = default_project.name();
                Some(
                    format!("/project/{default_project_name}/service/tls_certificate_provider")
                        .parse()?,
                )
            }
            InletTls::CustomTlsProvider {
                tls_certificate_provider,
            } => Some(tls_certificate_provider.parse()?),
        },
    };

    let result = node_manager
        .create_inlet(
            context,
            request.from,
            Route::default(),
            Route::default(),
            request.to,
            request.name.unwrap_or_else(random_string),
            request.allow,
            Some(request.retry_wait),
            request.authorized,
            false,
            request.identity,
            enable_udp_puncture,
            disable_tcp_fallback,
            privileged,
            tls_certificate_provider,
            false,
            false,
        )
        .await;
    match result {
        Ok(status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::CREATED,
            InletStatus::try_from(status)?,
        )?),
        Err(error) => {
            // TODO: specialize errors
            warn!("Failed to create tcp inlet: {:?}", error);
            let code = error.code();
            match code.kind {
                Kind::AlreadyExists => ControlApiHttpResponse::conflict(
                    "TCP Inlet with the same name or port already exists",
                ),
                _ => ControlApiHttpResponse::internal_error("Failed to create tcp inlet"),
            }
        }
    }
}

#[utoipa::path(
    patch,
    operation_id = "update_tcp_inlet",
    summary = "Update a TCP Inlet",
    description =
"Update a TCP Inlet given its name.

Only the `allow` policy expression can be updated.
To update any other field, delete the TCP Inlet and create a new one.",
    path = "/{node}/tcp-inlets/{name}",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully updated", body = InletStatus),
        (status = NOT_FOUND, description = "TCP Inlet not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Inlet name"),
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
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: UpdateInletRequest = common::parse_request_body(body)?;

    if node_manager.show_inlet(resource_id).await.is_none() {
        return ControlApiHttpResponse::not_found("TCP Inlet not found");
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
    description = "List all TCP Inlets created in a node regardless of their statuses.",
    path = "/{node}/tcp-inlets",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<InletStatus>),
    ),
    params(
        ("node" = NodeName,),
    )
)]
async fn handle_tcp_inlet_list(
    node_manager: &Arc<NodeManager>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let mut inlets: Vec<InletStatus> = Vec::new();
    for status in node_manager.list_inlets().await {
        inlets.push(InletStatus::try_from(status)?);
    }
    Ok(ControlApiHttpResponse::with_body(StatusCode::OK, inlets)?)
}

#[utoipa::path(
    delete,
    operation_id = "delete_tcp_inlet",
    summary = "Delete a TCP Inlet",
    description = "Delete a TCP Inlet given its name.",
    path = "/{node}/tcp-inlets/{name}",
    tags = ["Portals"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Inlet name"),
    )
)]
async fn handle_tcp_inlet_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let result = node_manager.delete_inlet(resource_id).await;
    match result {
        Ok(_) => Ok(ControlApiHttpResponse::without_body(
            StatusCode::NO_CONTENT,
        )?),
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
    description = "Get a TCP Inlet given its name.",
    path = "/{node}/tcp-inlets/{name}",
    tags = ["Portals"],
    responses(
        (status = OK, description = "Successfully retrieved", body = InletStatus),
        (status = NOT_FOUND, description = "TCP Inlet not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "TCP Inlet name")
    )
)]
async fn handle_tcp_inlet_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    match node_manager.show_inlet(resource_id).await {
        None => ControlApiHttpResponse::not_found("Inlet not found"),
        Some(status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::OK,
            InletStatus::try_from(status)?,
        )?),
    }
}

#[cfg(test)]
mod test {
    use crate::control_api::http::{ControlApiHttpRequest, ControlApiHttpResponse};
    use crate::control_api::protocol::common::{ConnectionStatus, ErrorResponse, HostnamePort};
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
            method: "POST".to_string(),
            uri: "/node-name/tcp-inlets".to_string(),
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
                    via: None,
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
        let bind_address = HostnamePort::try_from(inlet_status.bind_address.as_str())?;
        assert_eq!(bind_address.hostname, "127.0.0.1");
        assert!(bind_address.port > 0);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-inlets/inlet-name".to_string(),
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

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-inlets".to_string(),
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

        let request = ControlApiHttpRequest {
            method: "DELETE".to_string(),
            uri: "/node-name/tcp-inlets/inlet-name".to_string(),
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
            uri: "/node-name/tcp-inlets/inlet-name".to_string(),
            body: None,
        };

        let encoded_request = NeutralMessage::from(minicbor::to_vec(&request)?);
        let encoded_response: NeutralMessage = context
            .send_and_receive(address.clone(), encoded_request)
            .await?;

        let response: ControlApiHttpResponse = minicbor::decode(&encoded_response.into_vec())?;
        assert_eq!(response.status, 404);
        let body: ErrorResponse = serde_json::from_slice(response.body.as_slice()).unwrap();
        assert_eq!(body.message, "Inlet not found");

        let request = ControlApiHttpRequest {
            method: "GET".to_string(),
            uri: "/node-name/tcp-inlets".to_string(),
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
