use crate::kafka::outlet_controller::KafkaOutletController;
use alloc::sync::Arc;
use bytes::BytesMut;

use kafka_protocol::messages::request_header::RequestHeader;
use kafka_protocol::messages::{ApiKey, MetadataResponse, ResponseHeader};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;

use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Mutex;
use ockam_node::Context;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::str::FromStr;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use tinyvec::alloc;
use tracing::warn;

use crate::kafka::portal_worker::InterceptError;
use crate::kafka::protocol_aware::utils::decode_body;
use crate::kafka::protocol_aware::{CorrelationId, KafkaMessageInterceptor, RequestInfo};

/// Intercepts responses of type `Metadata` to extract the list of brokers
/// then creates an outlet for each of them through [`KafkaOutletController`]
#[derive(Clone)]
pub(crate) struct OutletInterceptorImpl {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    outlet_controller: KafkaOutletController,
    flow_control_id: FlowControlId,
}

impl OutletInterceptorImpl {
    pub(crate) fn new(
        outlet_controller: KafkaOutletController,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            request_map: Arc::new(Mutex::new(HashMap::new())),
            outlet_controller,
            flow_control_id,
        }
    }
}

#[async_trait]
impl KafkaMessageInterceptor for OutletInterceptorImpl {
    async fn intercept_request(
        &self,
        _context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        let mut buffer = original.peek_bytes(0..original.len());

        let api_key_num = buffer
            .peek_bytes(0..2)
            .try_get_i16()
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        let api_key = ApiKey::try_from(api_key_num).map_err(|_| {
            warn!("unknown request api: {api_key_num}");
            InterceptError::Io(Error::from(ErrorKind::InvalidData))
        })?;

        let version = buffer
            .peek_bytes(2..4)
            .try_get_i16()
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        let result = RequestHeader::decode(&mut buffer, api_key.request_header_version(version));
        let header = match result {
            Ok(header) => header,
            Err(_) => {
                //the error doesn't contain any useful information
                warn!("cannot decode request kafka header");
                return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
            }
        };

        let api_key = ApiKey::try_from(api_key_num).map_err(|_| {
            warn!("unknown request api: {api_key_num}");
            InterceptError::Io(Error::from(ErrorKind::InvalidData))
        })?;

        debug!(
            "request: length: {}, correlation {}, version {}, api {:?}",
            buffer.len(),
            header.correlation_id,
            header.request_api_version,
            api_key
        );

        if api_key == ApiKey::MetadataKey {
            self.request_map.lock().unwrap().insert(
                header.correlation_id,
                RequestInfo {
                    request_api_key: ApiKey::MetadataKey,
                    request_api_version: header.request_api_version,
                },
            );
        }

        Ok(original)
    }

    async fn intercept_response(
        &self,
        context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        let mut buffer = original.peek_bytes(0..original.len());

        //we can/need to decode only mapped requests
        let correlation_id = buffer
            .peek_bytes(0..4)
            .try_get_i32()
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        let result = self
            .request_map
            .lock()
            .unwrap()
            .get(&correlation_id)
            .cloned();

        if let Some(request_info) = result {
            let result = ResponseHeader::decode(
                &mut buffer,
                request_info
                    .request_api_key
                    .response_header_version(request_info.request_api_version),
            );

            let _header = match result {
                Ok(header) => header,
                Err(_) => {
                    //the error doesn't contain any useful information
                    warn!("cannot decode response kafka header");
                    return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
                }
            };

            debug!(
                "response: length: {}, correlation {}, version {}, api {:?}",
                buffer.len(),
                correlation_id,
                request_info.request_api_version,
                request_info.request_api_key
            );

            if request_info.request_api_key == ApiKey::MetadataKey {
                let response: MetadataResponse =
                    decode_body(&mut buffer, request_info.request_api_version)?;

                for (broker_id, metadata) in response.brokers {
                    let socket_addr = SocketAddr::from_str(
                        format!("{}:{}", metadata.host, metadata.port).as_str(),
                    )
                    .map_err(|e| {
                        InterceptError::Ockam(ockam_core::Error::new(
                            Origin::Ockam,
                            Kind::Invalid,
                            format!("cannot parse a socket address from the broker {broker_id:?} metadata {e:?}"),
                        ))
                    })?;
                    let outlet_address = self
                        .outlet_controller
                        .assert_outlet_for_broker(context, broker_id.0, socket_addr)
                        .await
                        .map_err(InterceptError::Ockam)?;

                    // allow the interceptor to reach the outlet
                    context
                        .flow_controls()
                        .add_consumer(outlet_address, &self.flow_control_id);
                }
            }
        } else {
            debug!(
                "response unmapped: length: {}, correlation {}",
                buffer.len(),
                correlation_id,
            );
        }
        Ok(original)
    }
}
