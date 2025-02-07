use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::relay::{CreateRelayRequest, RelayStatus};
use crate::nodes::models::relay::ReturnTiming;
use crate::nodes::NodeManager;
use http::StatusCode;
use ockam::identity::Identifier;
use ockam_core::compat::rand::random_string;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_relay(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => handle_relay_create(context, &self.node_manager, body).await,
            "GET" => match resource_id {
                None => handle_relay_list(&self.node_manager).await,
                Some(id) => handle_relay_get(&self.node_manager, id).await,
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => handle_relay_delete(&self.node_manager, id).await,
            },
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }
}

#[utoipa::path(
    put,
    operation_id = "create_relay",
    summary = "Create a new Relay",
    path = "/{node}/relay",
    tags = ["relay"],
    responses(
        (status = CREATED, description = "Successfully created", body = RelayStatus),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    ),
    request_body(
        content = CreateRelayRequest,
        content_type = "application/json",
        description = "Creation request"
    )
)]
async fn handle_relay_create(
    context: &Context,
    node_manager: &Arc<NodeManager>,
    body: Option<Vec<u8>>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let request: CreateRelayRequest = if let Some(body) = body {
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

    let to = if let Ok(to) = MultiAddr::try_from(request.to.as_str()) {
        to
    } else {
        warn!("Invalid 'to' address");
        return ControlApiHttpResponse::invalid_body();
    };

    let name = request.name.unwrap_or_else(random_string);
    let address = request.address.unwrap_or_else(|| name.clone());
    let authorized = if let Some(authorized) = request.authorized {
        let result = Identifier::try_from(authorized.as_str());
        match result {
            Ok(id) => Some(id),
            Err(error) => {
                warn!("Invalid authorized identity: {:?}", error);
                return ControlApiHttpResponse::invalid_body();
            }
        }
    } else {
        None
    };

    let result = node_manager
        .create_relay(
            context,
            &to,
            name,
            authorized,
            Some(address),
            ReturnTiming::Immediately,
        )
        .await;
    match result {
        Ok(status) => {
            ControlApiHttpResponse::with_body(StatusCode::CREATED, RelayStatus::from(status))
        }
        Err(error) => {
            // TODO: specialize errors
            // name already exists
            warn!("Failed to create Relay: {:?}", error);
            ControlApiHttpResponse::internal_error(error)
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "list_relay",
    summary = "List all Relays",
    path = "/{node}/relay",
    tags = ["relay"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<RelayStatus>),
    ),
    params(
        ("node" = String, description = "Destination node name"),
    )
)]
async fn handle_relay_list(
    node_manager: &Arc<NodeManager>,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let mut inlets: Vec<RelayStatus> = Vec::new();

    for status in node_manager.get_relays().await {
        inlets.push(RelayStatus::from(status));
    }

    ControlApiHttpResponse::with_body(StatusCode::OK, inlets)
}

#[utoipa::path(
    delete,
    operation_id = "delete_relay",
    summary = "Delete a Relay",
    path = "/{node}/relay/{resource_id}",
    tags = ["relay"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
    )
)]
async fn handle_relay_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    let result = node_manager.delete_relay_impl(resource_id).await;
    match result {
        Ok(_) => ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT),
        Err(error) => {
            warn!("Failed to delete Relay: {:?}", error);
            ControlApiHttpResponse::internal_error(error)
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "get_relay",
    summary = "Get a Relay",
    path = "/{node}/relay/{resource_id}",
    tags = ["relay"],
    responses(
        (status = OK, description = "Successfully retrieved", body = RelayStatus),
        (status = NOT_FOUND, description = "Resource not found"),
    ),
    params(
        ("node" = String, description = "Destination node name"),
        ("resource_id" = String, description = "Resource ID")
    )
)]
async fn handle_relay_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> ockam_core::Result<ControlApiHttpResponse> {
    match node_manager.show_relay(resource_id).await {
        None => ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND),
        Some(status) => {
            ControlApiHttpResponse::with_body(StatusCode::OK, RelayStatus::from(status))
        }
    }
}
