use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::protocol::outlet::{
    CreateOutletRequest, OutletKind, OutletStatus, OutletTls,
};
use crate::control_api::ControlApiHttpResponse;
use crate::nodes::models::portal::OutletAccessControl;
use http::StatusCode;
use ockam_abac::PolicyExpression;
use ockam_core::errcode::Kind;
use ockam_core::Address;
use ockam_node::Context;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_outlet(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => self.handle_tcp_outlet_create(context, body).await,
            "GET" => match resource_id {
                None => self.handle_tcp_outlet_list().await,
                Some(id) => self.handle_tcp_outlet_get(id).await,
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => self.handle_tcp_outlet_delete(id).await,
            },
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }

    async fn handle_tcp_outlet_create(
        &self,
        context: &Context,
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

        let result = self
            .node_manager
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
                Kind::AlreadyExists => {
                    ControlApiHttpResponse::with_body(StatusCode::CONFLICT, error.to_string())
                }
                _ => ControlApiHttpResponse::with_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error.to_string(),
                ),
            },
        }
    }

    async fn handle_tcp_outlet_list(&self) -> ockam_core::Result<ControlApiHttpResponse> {
        let outlets: Vec<OutletStatus> = self
            .node_manager
            .list_outlets()
            .into_iter()
            .map(OutletStatus::from)
            .collect();
        ControlApiHttpResponse::with_body(StatusCode::OK, outlets)
    }

    async fn handle_tcp_outlet_get(
        &self,
        resource_id: &str,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        let result = self
            .node_manager
            .show_outlet(&Address::from_string(resource_id));
        match result {
            None => ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND),
            Some(status) => {
                ControlApiHttpResponse::with_body(StatusCode::OK, OutletStatus::from(status))
            }
        }
    }

    async fn handle_tcp_outlet_delete(
        &self,
        resource_id: &str,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        self.node_manager
            .delete_outlet(&Address::from_string(resource_id))
            .await?;
        ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT)
    }
}

#[cfg(test)]
mod test {
    use crate::control_api::protocol::common::HostnamePort;
    use crate::control_api::protocol::outlet::{CreateOutletRequest, OutletKind, OutletStatus};
    use crate::control_api::{ControlApiHttpRequest, ControlApiHttpResponse};
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
