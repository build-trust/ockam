use crate::http::state::{ClientRequestWriter, RequestState};
use crate::nodes::models::services::{DeleteServiceRequest, StartServiceRequest};
use crate::nodes::registry::HttpHeaderInterceptorInfo;
use crate::nodes::{NodeManager, NodeManagerWorker};
use crate::DefaultAddress;
use httparse::Request;
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::{Action, PolicyAccessControl, Resource, ResourceType};
use ockam_core::api::Response;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};
use ockam_node::Context;
use ockam_transport_tcp::{
    read_portal_payload_length, Direction, PortalInletInterceptor, PortalInterceptor,
    PortalInterceptorFactory,
};
use std::io::Write;
use std::sync::{Arc, Mutex as SyncMutex};

/// An HTTP headers interceptor that rewrites the headers of the HTTP
/// request with the statically provided headers.
struct StaticHttpHeadersInterceptor {
    headers: Arc<Vec<(String, String)>>,
}

impl StaticHttpHeadersInterceptor {
    /// Starts a listener that will intercept data in a portal on the inlet side.
    /// The listener will rewrite the headers of the HTTP request with the provided headers.
    pub async fn start_listener(
        context: &Context,
        listener_address: Address,
        headers: Vec<(String, String)>,
        policy_access_control: Option<PolicyAccessControl>,
    ) -> ockam_core::Result<()> {
        let flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&Address::from_string(
                DefaultAddress::SECURE_CHANNEL_LISTENER,
            ))
            .ok_or_else(|| {
                ockam_core::Error::new(
                    Origin::Channel,
                    Kind::NotFound,
                    "Secure channel listener not found",
                )
            })?;

        context
            .flow_controls()
            .add_consumer(&listener_address, &flow_control_id);

        let incoming_access_control: Arc<dyn IncomingAccessControl>;
        let outgoing_access_control: Arc<dyn OutgoingAccessControl>;
        if let Some(policy_access_control) = policy_access_control {
            incoming_access_control = Arc::new(policy_access_control.create_incoming());
            outgoing_access_control = Arc::new(policy_access_control.create_outgoing(context)?);
        } else {
            incoming_access_control = Arc::new(AllowAll);
            outgoing_access_control = Arc::new(AllowAll);
        }

        PortalInletInterceptor::start_listener(
            context,
            listener_address,
            Arc::new(StaticHttpHeadersInterceptor {
                headers: Arc::new(headers),
            }),
            incoming_access_control,
            outgoing_access_control,
            read_portal_payload_length(),
        )
    }
}

impl PortalInterceptorFactory for StaticHttpHeadersInterceptor {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(HttpHeadersInterceptor {
            headers: self.headers.clone(),
            state: SyncMutex::new(RequestState::ParsingHeader(None)),
        })
    }
}

struct HttpHeadersInterceptor {
    headers: Arc<Vec<(String, String)>>,
    state: SyncMutex<RequestState>,
}

#[async_trait]
impl PortalInterceptor for HttpHeadersInterceptor {
    async fn intercept(
        &self,
        _context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>> {
        match direction {
            Direction::FromOutletToInlet => Ok(Some(buffer.to_vec())),
            Direction::FromInletToOutlet => {
                let mut guard = self.state.lock().unwrap();
                Ok(Some(guard.process_http_buffer(buffer, self)?))
            }
        }
    }
}

impl ClientRequestWriter for &HttpHeadersInterceptor {
    fn write_headers(&self, request: &Request, buffer: &mut Vec<u8>) -> ockam_core::Result<()> {
        write!(
            buffer,
            "{} {} HTTP/1.{}\r\n",
            request.method.unwrap(),
            request.path.unwrap(),
            request.version.unwrap()
        )
        .unwrap();

        for (name, value) in self.headers.iter() {
            write!(buffer, "{}: {}\r\n", name, value).unwrap();
        }

        for h in &*request.headers {
            if !self
                .headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(h.name))
            {
                write!(buffer, "{}: ", h.name).unwrap();
                buffer.extend_from_slice(h.value);
                buffer.extend_from_slice(b"\r\n");
            }
        }

        buffer.extend_from_slice(b"\r\n");
        Ok(())
    }
}

/// Request body to create a new HTTP rewrite headers interceptor
#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct HttpHeadersInterceptorRequest {
    #[n(0)] pub headers: Vec<(String, String)>,
}

impl NodeManagerWorker {
    pub async fn start_http_header_service(
        &self,
        request: StartServiceRequest<HttpHeadersInterceptorRequest>,
    ) -> ockam_core::Result<Response<()>, Response<ockam_core::api::Error>> {
        let result = self
            .node_manager
            .start_http_header_service(
                Address::from_string(request.address()),
                request.request().headers.clone(),
            )
            .await;

        match result {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub async fn delete_http_overwrite_header_service(
        &self,
        request: DeleteServiceRequest,
    ) -> ockam_core::Result<Response<()>, Response<ockam_core::api::Error>> {
        let result = self
            .node_manager
            .delete_http_overwrite_header_service(&Address::from_string(request.address()))
            .await;

        match result {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    pub async fn start_http_header_service(
        &self,
        listener_address: Address,
        headers: Vec<(String, String)>,
    ) -> ockam_core::Result<()> {
        let policy_access_control = if let Some(project_authority) = self.project_authority() {
            Some(
                self.policy_access_control(
                    Some(project_authority),
                    Resource::new(listener_address.to_string(), ResourceType::TcpInlet),
                    Action::HandleMessage,
                    None,
                )
                .await?,
            )
        } else {
            None
        };

        let context = self.ctx();
        StaticHttpHeadersInterceptor::start_listener(
            context,
            listener_address.clone(),
            headers,
            policy_access_control,
        )
        .await?;

        self.registry
            .http_headers_interceptors
            .insert(listener_address, HttpHeaderInterceptorInfo {});

        Ok(())
    }

    pub async fn delete_http_overwrite_header_service(
        &self,
        listener_address: &Address,
    ) -> ockam_core::Result<()> {
        let context = self.ctx();
        context.stop_address(listener_address)?;

        self.registry
            .http_headers_interceptors
            .remove(listener_address);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::nodes::service::{NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions};
    use crate::test_utils::start_manager_for_tests;
    use ockam_core::NeutralMessage;
    use ockam_transport_tcp::PortalMessage;

    #[ockam::test]
    async fn main(context: &mut Context) -> ockam::Result<()> {
        let handler = start_manager_for_tests(
            context,
            None,
            Some(NodeManagerTrustOptions::new(
                NodeManagerCredentialRetrieverOptions::None,
                NodeManagerCredentialRetrieverOptions::None,
                None,
                NodeManagerCredentialRetrieverOptions::None,
            )),
        )
        .await?;

        StaticHttpHeadersInterceptor::start_listener(
            context,
            "http_interceptor".into(),
            vec![("Host".to_string(), "ockam.io".to_string())],
            None,
        )
        .await?;

        let connection = handler
            .node_manager
            .make_connection(
                &format!(
                    "/service/http_interceptor/service/{}",
                    context.primary_address().address()
                )
                .parse()?,
                handler.node_manager.identifier(),
                None,
                None,
            )
            .await?;

        let route = connection.route()?;

        context
            .send(route.clone(), PortalMessage::Ping.to_neutral_message()?)
            .await?;

        let _ = context.receive::<NeutralMessage>().await?;

        context
            .send(
                route.clone(),
                PortalMessage::Payload(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n", None)
                    .to_neutral_message()?,
            )
            .await?;

        let message = context.receive::<NeutralMessage>().await?;
        let message = PortalMessage::decode(message.payload())?;

        if let PortalMessage::Payload(payload, _) = message {
            let message = String::from_utf8(payload.to_vec()).unwrap();
            assert_eq!(message, "GET / HTTP/1.1\r\nHost: ockam.io\r\n\r\n");
        } else {
            panic!("Decoded message is not a Payload");
        }

        Ok(())
    }
}
