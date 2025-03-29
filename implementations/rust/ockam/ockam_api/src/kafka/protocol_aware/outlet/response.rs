use crate::kafka::protocol_aware::outlet::OutletInterceptorImpl;
use crate::kafka::protocol_aware::utils::decode_body;
use crate::kafka::protocol_aware::{InterceptError, KafkaMessageResponseInterceptor};
use bytes::BytesMut;
use kafka_protocol::messages::{ApiKey, MetadataResponse, ResponseHeader};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
impl KafkaMessageResponseInterceptor for OutletInterceptorImpl {
    async fn intercept_response(
        &self,
        context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        let mut buffer = original.peek_bytes(0..original.len());

        // we can/need to decode only mapped requests
        let correlation_id = buffer.peek_bytes(0..4).try_get_i32()?;

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
                    // the error doesn't contain any useful information
                    warn!("cannot decode response kafka header");
                    return Err(InterceptError::InvalidData);
                }
            };

            debug!(
                "response: length: {}, correlation {}, version {}, api {:?}",
                buffer.len(),
                correlation_id,
                request_info.request_api_version,
                request_info.request_api_key
            );

            // the responses of metadata request contain the list of brokers addresses,
            // we override them with the outlets addresses
            if request_info.request_api_key == ApiKey::Metadata {
                let response: MetadataResponse =
                    decode_body(&mut buffer, request_info.request_api_version)?;

                for broker in response.brokers {
                    let address = format!("{}:{}", broker.host.as_str(), broker.port);
                    let outlet_address = self
                        .outlet_controller
                        .assert_outlet_for_broker(context, broker.node_id.0, address)
                        .await?;

                    // allow the interceptor to reach the outlet
                    context
                        .flow_controls()
                        .add_consumer(&outlet_address, &self.flow_control_id);
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
