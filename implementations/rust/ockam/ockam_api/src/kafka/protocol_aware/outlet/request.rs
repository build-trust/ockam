use crate::kafka::protocol_aware::outlet::OutletInterceptorImpl;
use crate::kafka::protocol_aware::{InterceptError, KafkaMessageInterceptorRequest, RequestInfo};
use bytes::BytesMut;
use kafka_protocol::messages::{ApiKey, RequestHeader};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;
use ockam_core::async_trait;
use ockam_node::Context;
use std::io::{Error, ErrorKind};

#[async_trait]
impl KafkaMessageInterceptorRequest for OutletInterceptorImpl {
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
                // the error doesn't contain any useful information
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
}
