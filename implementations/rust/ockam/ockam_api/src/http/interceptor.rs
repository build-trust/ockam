use crate::http::state::HttpState;
use crate::nodes::models::services::{DeleteServiceRequest, StartServiceRequest};
use crate::nodes::registry::HttpHeaderInterceptorInfo;
use crate::nodes::{NodeManager, NodeManagerWorker};
use crate::DefaultAddress;
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::{Action, PolicyAccessControl, Resource, ResourceType};
use ockam_core::api::Response;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};
use ockam_node::Context;
use ockam_transport_tcp::{
    read_portal_payload_length, Direction, PortalInletInterceptor, PortalInterceptor,
    PortalInterceptorFactory, PortalOutletInterceptor,
};
use std::sync::{Arc, Mutex as SyncMutex};

pub trait HttpHeaderProvider: Sync + Send + 'static {
    fn read_headers(&self) -> ockam_core::Result<Vec<(String, String)>>;
}

pub(crate) struct StaticHttpHeaderProvider {
    headers: Vec<(String, String)>,
}

impl StaticHttpHeaderProvider {
    pub fn new(headers: Vec<(String, String)>) -> Arc<Self> {
        Arc::new(Self { headers })
    }
}

impl HttpHeaderProvider for StaticHttpHeaderProvider {
    fn read_headers(&self) -> ockam_core::Result<Vec<(String, String)>> {
        Ok(self.headers.clone())
    }
}

pub(crate) struct HttpInterceptorFactory {
    header_provider: Arc<dyn HttpHeaderProvider>,
    direction: Direction,
}

impl HttpInterceptorFactory {
    pub async fn create_outlet_interceptor(
        context: &Context,
        listener_address: Address,
        outlet_address: Address,
        header_provider: Arc<dyn HttpHeaderProvider>,
        direction: Direction,
        policy_access_control: Option<PolicyAccessControl>,
    ) -> ockam_core::Result<()> {
        let flow_controls = context.flow_controls();

        let default_secure_channel_listener_flow_control_id = flow_controls
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

        let spawner_flow_control_id = FlowControls::generate_flow_control_id();

        let incoming_access_control: Arc<dyn IncomingAccessControl>;
        let outgoing_access_control: Arc<dyn OutgoingAccessControl>;
        if let Some(policy_access_control) = policy_access_control {
            incoming_access_control = Arc::new(policy_access_control.create_incoming());
            outgoing_access_control = Arc::new(policy_access_control.create_outgoing(context)?);
        } else {
            incoming_access_control = Arc::new(AllowAll);
            outgoing_access_control = Arc::new(AllowAll);
        }
        PortalOutletInterceptor::create(
            context,
            listener_address.clone(),
            Some(spawner_flow_control_id.clone()),
            Arc::new(HttpInterceptorFactory {
                direction,
                header_provider,
            }),
            outgoing_access_control,
            incoming_access_control,
            read_portal_payload_length(),
        )?;

        // Every secure channel can reach the listener
        flow_controls.add_consumer(
            &listener_address,
            &default_secure_channel_listener_flow_control_id,
        );

        // Mark the listener address as a spawner
        flow_controls.add_spawner(&listener_address, &spawner_flow_control_id);

        // Allows any spawned worker to reach the outlet
        flow_controls.add_consumer(&outlet_address, &spawner_flow_control_id);

        Ok(())
    }
    pub async fn create_inlet_interceptor(
        context: &Context,
        listener_address: Address,
        header_provider: Arc<dyn HttpHeaderProvider>,
        direction: Direction,
        policy_access_control: Option<PolicyAccessControl>,
    ) -> ockam_core::Result<()> {
        let incoming_access_control: Arc<dyn IncomingAccessControl>;
        let outgoing_access_control: Arc<dyn OutgoingAccessControl>;
        if let Some(policy_access_control) = policy_access_control {
            incoming_access_control = Arc::new(policy_access_control.create_incoming());
            outgoing_access_control = Arc::new(policy_access_control.create_outgoing(context)?);
        } else {
            incoming_access_control = Arc::new(AllowAll);
            outgoing_access_control = Arc::new(AllowAll);
        }

        PortalInletInterceptor::create(
            context,
            listener_address,
            Arc::new(HttpInterceptorFactory {
                direction,
                header_provider,
            }),
            incoming_access_control,
            outgoing_access_control,
            read_portal_payload_length(),
        )
    }
}

impl PortalInterceptorFactory for HttpInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(HttpClientHeadersInterceptor {
            header_provider: self.header_provider.clone(),
            direction: self.direction,
            state: SyncMutex::new(HttpState::ParsingHeader(None)),
        })
    }
}

struct HttpClientHeadersInterceptor {
    header_provider: Arc<dyn HttpHeaderProvider>,
    state: SyncMutex<HttpState>,
    direction: Direction,
}

#[async_trait]
impl PortalInterceptor for HttpClientHeadersInterceptor {
    async fn intercept(
        &self,
        _context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>> {
        match self.direction {
            Direction::FromInletToOutlet => match &direction {
                Direction::FromOutletToInlet => Ok(Some(buffer.to_vec())),
                Direction::FromInletToOutlet => {
                    let mut guard = self.state.lock().unwrap();
                    Ok(Some(guard.process_http_buffer(
                        buffer,
                        Direction::FromInletToOutlet,
                        self.header_provider.as_ref(),
                    )?))
                }
            },
            Direction::FromOutletToInlet => match direction {
                Direction::FromOutletToInlet => {
                    let mut guard = self.state.lock().unwrap();
                    Ok(Some(guard.process_http_buffer(
                        buffer,
                        Direction::FromOutletToInlet,
                        self.header_provider.as_ref(),
                    )?))
                }
                Direction::FromInletToOutlet => Ok(Some(buffer.to_vec())),
            },
        }
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
        context: &Context,
        request: StartServiceRequest<HttpHeadersInterceptorRequest>,
    ) -> ockam_core::Result<Response<()>, Response<ockam_core::api::Error>> {
        let result = self
            .node_manager
            .start_inlet_http_header_service(
                context,
                Address::from_string(request.address()),
                StaticHttpHeaderProvider::new(request.request().headers.clone()),
                Direction::FromInletToOutlet,
            )
            .await;

        match result {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub async fn delete_http_overwrite_header_service(
        &self,
        context: &Context,
        request: DeleteServiceRequest,
    ) -> ockam_core::Result<Response<()>, Response<ockam_core::api::Error>> {
        let result = self
            .node_manager
            .delete_http_overwrite_header_service(context, &Address::from_string(request.address()))
            .await;

        match result {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    pub async fn start_outlet_http_header_service(
        &self,
        context: &Context,
        listener_address: Address,
        outlet_address: Address,
        headers_provider: Arc<dyn HttpHeaderProvider>,
        direction: Direction,
    ) -> ockam_core::Result<()> {
        let policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(listener_address.to_string(), ResourceType::TcpOutlet),
                Action::HandleMessage,
                None,
            )
            .await?;

        HttpInterceptorFactory::create_outlet_interceptor(
            context,
            listener_address.clone(),
            outlet_address,
            headers_provider,
            direction,
            Some(policy_access_control),
        )
        .await?;

        self.registry
            .http_headers_interceptors
            .insert(listener_address, HttpHeaderInterceptorInfo {});

        Ok(())
    }

    pub async fn start_inlet_http_header_service(
        &self,
        context: &Context,
        listener_address: Address,
        headers_provider: Arc<dyn HttpHeaderProvider>,
        direction: Direction,
    ) -> ockam_core::Result<()> {
        let policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(listener_address.to_string(), ResourceType::TcpInlet),
                Action::HandleMessage,
                None,
            )
            .await?;

        HttpInterceptorFactory::create_inlet_interceptor(
            context,
            listener_address.clone(),
            headers_provider,
            direction,
            Some(policy_access_control),
        )
        .await?;

        self.registry
            .http_headers_interceptors
            .insert(listener_address, HttpHeaderInterceptorInfo {});

        Ok(())
    }

    pub async fn delete_http_overwrite_header_service(
        &self,
        context: &Context,
        listener_address: &Address,
    ) -> ockam_core::Result<()> {
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
    use crate::http::interceptor::HttpHeaderProvider;
    use crate::http::state::HttpState;
    use crate::nodes::service::{NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions};
    use crate::test_utils::start_manager_for_tests;
    use ockam_core::NeutralMessage;
    use ockam_transport_tcp::Direction;
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

        HttpInterceptorFactory::create_inlet_interceptor(
            context,
            "http_interceptor".into(),
            StaticHttpHeaderProvider::new(vec![("Host".to_string(), "ockam.io".to_string())]),
            Direction::FromInletToOutlet,
            None,
        )
        .await?;

        let connection = handler
            .node_manager
            .make_connection(
                context,
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

    struct EmptyHeaderProvider;
    impl HttpHeaderProvider for EmptyHeaderProvider {
        fn read_headers(&self) -> ockam_core::Result<Vec<(String, String)>> {
            Ok(vec![])
        }
    }

    const REQ: &str = "POST / HTTP/1.1\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Transfer-Encoding: gzip, chunked\r\n\r\n\
4\r\nWiki\r\n7\r\npedia i\r\n0\r\n\r\n";

    const TOKEN: &str = "SAMPLE-TOKEN";

    const EXPECTED: &str = "POST / HTTP/1.1\r\n\
Authorization: Token SAMPLE-TOKEN\r\n\
Host: www.example.com\r\n\
User-Agent: Mozilla/5.0\r\n\
Accept-Encoding: gzip, deflate, br\r\n\
Transfer-Encoding: gzip, chunked\r\n\r\n\
4\r\nWiki\r\n7\r\npedia i\r\n0\r\n\r\n";

    #[test]
    fn parse_post_with_chunked_transfers() {
        let mut data = Vec::new();
        data.extend_from_slice(REQ.as_bytes());
        data.extend_from_slice(REQ.as_bytes());

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = HttpState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, Direction::FromInletToOutlet, &EmptyHeaderProvider)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(
                String::from_utf8(result).unwrap(),
                EXPECTED.to_owned() + EXPECTED
            );
            assert_eq!(request_state, HttpState::ParsingHeader(None));
        }
    }

    #[test]
    fn parse_post_with_content_length() {
        let req = "POST /test HTTP/1.1\r\n\
Host: foo.example\r\n\
Content-Type: application/x-www-form-urlencoded\r\n\
Content-Length: 27\r\n\r\n\
field1=value1&field2=value2";
        let expected_r = format!(
            "POST /test HTTP/1.1\r\n\
Authorization: Token {}\r\n\
Host: foo.example\r\n\
Content-Type: application/x-www-form-urlencoded\r\n\
Content-Length: 27\r\n\r\n\
field1=value1&field2=value2",
            TOKEN
        );

        let data = [req.as_bytes(), req.as_bytes()].concat();
        let expected = [expected_r.as_bytes(), expected_r.as_bytes()].concat();

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = HttpState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, Direction::FromInletToOutlet, &EmptyHeaderProvider)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(
                String::from_utf8(result).unwrap(),
                String::from_utf8(expected.clone()).unwrap()
            );
            assert_eq!(request_state, HttpState::ParsingHeader(None));
        }
    }

    #[test]
    fn parse_get_requests() {
        let req = "GET /home/user/example.txt HTTP/1.1\r\n\r\n";
        let mut data = Vec::new();
        data.extend_from_slice(req.as_bytes());
        data.extend_from_slice(req.as_bytes());

        let mut expected = format!(
            "GET /home/user/example.txt HTTP/1.1\r\nAuthorization: Token {}\r\n\r\n",
            TOKEN
        );
        expected = expected.clone() + &expected;

        for size in [1, 5, 32, 1024] {
            let mut result = Vec::new();
            let mut request_state = HttpState::ParsingHeader(None);
            for chunk in data.chunks(size) {
                let data_out = request_state
                    .process_http_buffer(chunk, Direction::FromInletToOutlet, &EmptyHeaderProvider)
                    .unwrap();
                result.extend_from_slice(&data_out);
            }
            assert_eq!(String::from_utf8(result).unwrap(), expected);
            assert_eq!(request_state, HttpState::ParsingHeader(None));
        }
    }
}
