use bytes::BytesMut;
use kafka_protocol::messages::{ApiKey, MetadataResponse, RequestHeader, ResponseHeader};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use tracing::info;

use ockam_core::compat::collections::HashMap;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::io::{Error, ErrorKind};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_node::Context;

use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::portal_worker::InterceptError;

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

#[derive(Clone, Debug)]
pub(crate) struct ProtocolState {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
}

impl ProtocolState {
    pub(crate) fn new() -> ProtocolState {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
        }
    }

    ///Parse request and map request <=> response
    /// fails if anything in the parsing fails to avoid leaking clear text payloads
    pub(crate) fn intercept_request(
        &self,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        //let's clone the view of the buffer without cloning the content
        let mut buffer = original.peek_bytes(0..original.len());

        let version = buffer
            .peek_bytes(2..4)
            .try_get_i16()
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        let result = RequestHeader::decode(&mut buffer, version);
        let header = match result {
            Ok(header) => header,
            Err(_) => {
                //the error doesn't contain any useful information
                warn!("cannot decode request kafka header");
                return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
            }
        };

        if let Ok(api_key) = ApiKey::try_from(header.request_api_key) {
            info!(
                "request: length: {}, version {:?}, api {:?}",
                buffer.len(),
                header.request_api_version,
                api_key
            );

            #[allow(clippy::single_match)]
            match api_key {
                ApiKey::MetadataKey => {
                    self.request_map.lock().unwrap().insert(
                        header.correlation_id,
                        RequestInfo {
                            request_api_key: api_key,
                            request_api_version: header.request_api_version,
                        },
                    );
                }
                _ => {}
            }
        } else {
            warn!("unknown request api: {:?}", header.request_api_key);
            return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
        }

        Ok(original)
    }

    pub(crate) async fn intercept_response(
        &self,
        context: &mut Context,
        mut original: BytesMut,
        inlet_map: &KafkaInletMap,
    ) -> Result<BytesMut, InterceptError> {
        //let's clone the view of the buffer without cloning the content
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
            let result = ResponseHeader::decode(&mut buffer, request_info.request_api_version);
            let header = match result {
                Ok(header) => header,
                Err(_) => {
                    //the error doesn't contain any useful information
                    warn!("cannot decode response kafka header");
                    return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
                }
            };

            info!(
                "response: length: {}, version {:?}, api {:?}",
                buffer.len(),
                request_info.request_api_version,
                request_info.request_api_key
            );

            #[allow(clippy::single_match)]
            match request_info.request_api_key {
                ApiKey::MetadataKey => {
                    let result =
                        MetadataResponse::decode(&mut buffer, request_info.request_api_version);
                    let mut response = match result {
                        Ok(response) => response,
                        Err(_) => {
                            warn!("cannot decode kafka message");
                            return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
                        }
                    };

                    info!("metadata response before: {:?}", &response);

                    for (broker_id, info) in &mut response.brokers {
                        let inlet_address: SocketAddr = inlet_map
                            .assert_inlet_for_broker(context, broker_id.0)
                            .await
                            .map_err(InterceptError::Ockam)?;

                        trace!(
                            "inlet_address: {} for broker {}",
                            &inlet_address,
                            broker_id.0
                        );

                        let ip_address = inlet_address.ip().to_string();
                        //TryFrom is broken, ugly but effective
                        info.host = unsafe {
                            StrBytes::from_utf8_unchecked(bytes::Bytes::from(ip_address))
                        };
                        info.port = inlet_address.port() as i32;
                    }
                    info!("metadata response after: {:?}", &response);

                    let mut modified_buffer = BytesMut::new();

                    header
                        .encode(&mut modified_buffer, request_info.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
                    response
                        .encode(&mut modified_buffer, request_info.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    return Ok(modified_buffer);
                }
                _ => {}
            }
        }

        Ok(original)
    }
}
