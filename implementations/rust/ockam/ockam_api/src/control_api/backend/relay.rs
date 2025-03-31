use crate::control_api::backend::common;
use crate::control_api::backend::common::ResourceKind;
use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::http::ControlApiHttpResponse;
use crate::control_api::protocol::common::{ErrorResponse, NodeName};
use crate::control_api::protocol::relay::{CreateRelayRequest, RelayStatus};
use crate::control_api::ControlApiError;
use crate::nodes::models::relay::ReturnTiming;
use crate::nodes::NodeManager;
use http::{Method, StatusCode};
use ockam::identity::Identifier;
use ockam_core::compat::rand::random_string;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::sync::Arc;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_relay(
        &self,
        context: &Context,
        method: Method,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<ControlApiHttpResponse, ControlApiError> {
        match method {
            Method::POST => handle_relay_create(context, &self.node_manager, body).await,
            Method::GET => match resource_id {
                None => handle_relay_list(&self.node_manager).await,
                Some(id) => handle_relay_get(&self.node_manager, id).await,
            },
            Method::DELETE => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(ResourceKind::Relays),
                Some(id) => handle_relay_delete(&self.node_manager, id).await,
            },
            _ => {
                warn!("Invalid method: {method}");
                ControlApiHttpResponse::invalid_method(
                    method,
                    vec![Method::POST, Method::GET, Method::DELETE],
                )
            }
        }
    }
}

#[utoipa::path(
    post,
    operation_id = "create_relay",
    summary = "Create a Relay",
    description =
"The main parameters are the destination node `to` and the `name`.
By default, the address inherits the name value, but it's possible to specify it to allow the creation
of multiple relays with the same address on different nodes.

The creation will be asynchronous and the initial status will be `down`.

Note that, to allow the relay creation in the destination node, the caller identity credential must
have an `ockam-relay` attribute set with the relay name.",
    path = "/{node}/relays",
    tags = ["Relays"],
    responses(
        (status = CREATED, description = "Successfully created", body = RelayStatus),
    ),
    params(
        ("node" = NodeName,),
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
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let request: CreateRelayRequest = common::parse_request_body(body)?;

    let to = match MultiAddr::try_from(request.to.as_str()) {
        Ok(to) => to,
        Err(error) => {
            warn!("Invalid 'to' address: {error:?}");
            return ControlApiHttpResponse::invalid_body();
        }
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
        Ok(status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::CREATED,
            RelayStatus::from(status),
        )?),
        Err(error) => {
            // TODO: specialize errors
            // name already exists
            warn!("Failed to create Relay: {:?}", error);
            ControlApiHttpResponse::internal_error("Failed to create Relay")
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "list_relay",
    summary = "List all Relays",
    description = "List all Relays created in the node regardless of their status.",
    path = "/{node}/relays",
    tags = ["Relays"],
    responses(
        (status = OK, description = "Successfully listed", body = Vec<RelayStatus>),
    ),
    params(
        ("node" = NodeName,),
    )
)]
async fn handle_relay_list(
    node_manager: &Arc<NodeManager>,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let mut inlets: Vec<RelayStatus> = Vec::new();

    for status in node_manager.get_relays().await {
        inlets.push(RelayStatus::from(status));
    }

    Ok(ControlApiHttpResponse::with_body(StatusCode::OK, inlets)?)
}

#[utoipa::path(
    delete,
    operation_id = "delete_relay",
    summary = "Delete a Relay",
    description = "Delete a Relay given its name.",
    path = "/{node}/relays/{name}",
    tags = ["Relays"],
    responses(
        (status = NO_CONTENT, description = "Successfully deleted"),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "Relay name"),
    )
)]
async fn handle_relay_delete(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    let result = node_manager.delete_relay_impl(resource_id).await;
    match result {
        Ok(_) => Ok(ControlApiHttpResponse::without_body(
            StatusCode::NO_CONTENT,
        )?),
        Err(error) => {
            warn!("Failed to delete Relay: {:?}", error);
            ControlApiHttpResponse::internal_error("Failed to delete Relay")
        }
    }
}

#[utoipa::path(
    get,
    operation_id = "get_relay",
    summary = "Get a Relay",
    description = "Get a Relay given its name.",
    path = "/{node}/relays/{name}",
    tags = ["Relays"],
    responses(
        (status = OK, description = "Successfully retrieved", body = RelayStatus),
        (status = NOT_FOUND, description = "Relay not found", body = ErrorResponse),
    ),
    params(
        ("node" = NodeName,),
        ("name" = String, description = "Relay name"),
    )
)]
async fn handle_relay_get(
    node_manager: &Arc<NodeManager>,
    resource_id: &str,
) -> Result<ControlApiHttpResponse, ControlApiError> {
    match node_manager.show_relay(resource_id).await {
        None => ControlApiHttpResponse::not_found("Relay not found"),
        Some(status) => Ok(ControlApiHttpResponse::with_body(
            StatusCode::OK,
            RelayStatus::from(status),
        )?),
    }
}
