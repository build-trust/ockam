use crate::kafka::protocol_aware::outlet::OutletInterceptorImpl;
use crate::kafka::protocol_aware::{InterceptError, KafkaMessageRequestInterceptor, RequestInfo};
use bytes::BytesMut;
use kafka_protocol::messages::{ApiKey, RequestHeader};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
impl KafkaMessageRequestInterceptor for OutletInterceptorImpl {
    async fn intercept_request(
        &self,
        _context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        // Inside the request we can find the api key (kind of request), the protocol version of the request,
        // and the identifier of the request.
        // The request identifier, called correlation id, that we can use to map the request
        // to the response.
        // for more information see:
        // https://cwiki.apache.org/confluence/display/KAFKA/A+Guide+To+The+Kafka+Protocol#AGuideToTheKafkaProtocol-Requests

        let mut buffer = original.peek_bytes(0..original.len());
        let api_key_num = buffer.peek_bytes(0..2).try_get_i16()?;
        let api_key = ApiKey::try_from(api_key_num).map_err(|_| {
            warn!("unknown request api: {api_key_num}");
            InterceptError::InvalidData
        })?;

        let version = buffer.peek_bytes(2..4).try_get_i16()?;

        let result = RequestHeader::decode(&mut buffer, api_key.request_header_version(version));
        let header = match result {
            Ok(header) => header,
            Err(_) => {
                // the error doesn't contain any useful information
                warn!("cannot decode request kafka header");
                return Err(InterceptError::InvalidData);
            }
        };

        debug!(
            "request: length: {}, correlation {}, version {}, api {:?}",
            buffer.len(),
            header.correlation_id,
            header.request_api_version,
            api_key
        );

        // we only need to keep track of the metadata request/response
        // to dynamically create an outlet for each broker
        if api_key == ApiKey::Metadata {
            self.request_map.lock().unwrap().insert(
                header.correlation_id,
                RequestInfo {
                    request_api_key: ApiKey::Metadata,
                    request_api_version: header.request_api_version,
                },
            );
        }

        Ok(original)
    }
}
