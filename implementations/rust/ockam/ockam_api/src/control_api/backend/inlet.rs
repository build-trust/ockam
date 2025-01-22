use crate::control_api::backend::entrypoint::HttpControlNodeApiBackend;
use crate::control_api::protocol::inlet::InletStatus;
use crate::control_api::protocol::inlet::{CreateInletRequest, InletKind, InletTls};
use crate::control_api::ControlApiHttpResponse;
use http::StatusCode;
use ockam_abac::PolicyExpression;
use ockam_core::compat::rand::random_string;
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

impl HttpControlNodeApiBackend {
    pub(super) async fn handle_tcp_inlet(
        &self,
        context: &Context,
        method: &str,
        resource_id: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match method {
            "PUT" => self.handle_tcp_inlet_create(context, body).await,
            "GET" => match resource_id {
                None => self.handle_tcp_inlet_list().await,
                Some(id) => self.handle_tcp_inlet_get(id).await,
            },
            "DELETE" => match resource_id {
                None => ControlApiHttpResponse::missing_resource_id(),
                Some(id) => self.handle_tcp_inlet_delete(id).await,
            },
            _ => ControlApiHttpResponse::invalid_method(),
        }
    }

    async fn handle_tcp_inlet_create(
        &self,
        context: &Context,
        body: Option<Vec<u8>>,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        let request: CreateInletRequest = if let Some(body) = body {
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
            InletTls::ProjectTls => {
                Some("/project/default/service/tls_certificate_provider".parse()?)
            }
            InletTls::CustomTlsProvider {
                tls_certificate_provider,
            } => Some(tls_certificate_provider.parse()?),
        };

        let authorized = match request.authorized {
            None => None,
            Some(authorized) => Some(authorized.parse()?),
        };

        let result = self
            .node_manager
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
            )
            .await;
        match result {
            Ok(status) => ControlApiHttpResponse::with_body(
                StatusCode::CREATED,
                InletStatus::try_from(status)?,
            ),
            Err(error) => {
                // TODO: specialize errors
                // name already exists
                // port already bound
                warn!("Failed to create tcp inlet: {:?}", error);
                ControlApiHttpResponse::internal_error()
            }
        }
    }

    async fn handle_tcp_inlet_list(&self) -> ockam_core::Result<ControlApiHttpResponse> {
        let mut inlets: Vec<InletStatus> = Vec::new();

        for status in self.node_manager.list_inlets().await {
            inlets.push(InletStatus::try_from(status)?);
        }

        ControlApiHttpResponse::with_body(StatusCode::OK, inlets)
    }

    async fn handle_tcp_inlet_delete(
        &self,
        resource_id: &str,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        let result = self.node_manager.delete_inlet(resource_id).await;
        match result {
            Ok(_) => ControlApiHttpResponse::without_body(StatusCode::NO_CONTENT),
            Err(error) => {
                warn!("Failed to delete tcp inlet: {:?}", error);
                ControlApiHttpResponse::internal_error()
            }
        }
    }

    async fn handle_tcp_inlet_get(
        &self,
        resource_id: &str,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        match self.node_manager.show_inlet(resource_id).await {
            None => ControlApiHttpResponse::without_body(StatusCode::NOT_FOUND),
            Some(status) => {
                ControlApiHttpResponse::with_body(StatusCode::OK, InletStatus::try_from(status)?)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::control_api::protocol::common::HostnamePort;
    use crate::control_api::protocol::inlet::{ConnectionStatus, CreateInletRequest, InletStatus};
    use crate::control_api::{ControlApiHttpRequest, ControlApiHttpResponse};
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
