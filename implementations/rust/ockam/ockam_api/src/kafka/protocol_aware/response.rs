use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use kafka_protocol::messages::fetch_response::FetchResponse;
use kafka_protocol::messages::find_coordinator_response::FindCoordinatorResponse;
use kafka_protocol::messages::metadata_response::MetadataResponse;
use kafka_protocol::messages::response_header::ResponseHeader;
use kafka_protocol::messages::ApiKey;
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::decode::Decoder;
use ockam_node::Context;
use tracing::{info, trace, warn};

use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::portal_worker::InterceptError;
use crate::kafka::protocol_aware::utils::{decode, encode, string_to_str_bytes};
use crate::kafka::protocol_aware::{Interceptor, MessageWrapper, RequestInfo};

impl Interceptor {
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
                "response: length: {}, correlation {}, version {}, api {:?}",
                buffer.len(),
                correlation_id,
                request_info.request_api_version,
                request_info.request_api_key
            );

            match request_info.request_api_key {
                ApiKey::FetchKey => {
                    return self
                        .handle_fetch_response(context, &mut buffer, &request_info, &header)
                        .await;
                }

                ApiKey::FindCoordinatorKey => {
                    return self
                        .handle_find_coordinator_response(
                            context,
                            &mut buffer,
                            inlet_map,
                            &request_info,
                            &header,
                        )
                        .await;
                }

                ApiKey::MetadataKey => {
                    return self
                        .handle_metadata_response(
                            context,
                            &mut buffer,
                            inlet_map,
                            request_info,
                            &header,
                        )
                        .await;
                }
                _ => {}
            }
        } else {
            info!(
                "response unmapped: length: {}, correlation {}",
                buffer.len(),
                correlation_id,
            );
        }

        Ok(original)
    }

    //for metadata we want to replace broker address and port
    // to dedicated tcp inlet ports
    async fn handle_metadata_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        inlet_map: &KafkaInletMap,
        request_info: RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: MetadataResponse = decode(buffer, request_info.request_api_version)?;

        //we need to keep a map of topic uuid to topic name since fetch
        //operations only use uuid
        if request_info.request_api_version >= 10 {
            for (topic_name, topic) in &response.topics {
                let topic_id = topic.topic_id.to_string();
                let topic_name = topic_name.to_string();

                trace!("metadata adding to map: {topic_id} => {topic_name}");
                self.uuid_to_name
                    .lock()
                    .unwrap()
                    .insert(topic_id, topic_name);
            }
        }

        trace!("metadata response before: {:?}", &response);

        for (broker_id, info) in response.brokers.iter_mut() {
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
            info.host = string_to_str_bytes(ip_address);
            info.port = inlet_address.port() as i32;
        }
        trace!("metadata response after: {:?}", &response);

        encode(header, &response, request_info.request_api_version)
    }

    async fn handle_find_coordinator_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        inlet_map: &KafkaInletMap,
        request_info: &RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: FindCoordinatorResponse =
            decode(buffer, request_info.request_api_version)?;

        //similarly to metadata, we want to expressed the coordinator using
        //local sidecar ip address
        //the format changed to array since version 4
        if request_info.request_api_version >= 4 {
            for coordinator in response.coordinators.iter_mut() {
                let inlet_address: SocketAddr = inlet_map
                    .assert_inlet_for_broker(context, coordinator.node_id.0)
                    .await
                    .map_err(InterceptError::Ockam)?;

                let ip_address = inlet_address.ip().to_string();
                coordinator.host = string_to_str_bytes(ip_address);
                coordinator.port = inlet_address.port() as i32;
            }
        } else {
            let inlet_address: SocketAddr = inlet_map
                .assert_inlet_for_broker(context, response.node_id.0)
                .await
                .map_err(InterceptError::Ockam)?;

            let ip_address = inlet_address.ip().to_string();
            response.host = string_to_str_bytes(ip_address);
            response.port = inlet_address.port() as i32;
        }

        encode(header, &response, request_info.request_api_version)
    }

    async fn handle_fetch_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        request_info: &RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: FetchResponse = decode(buffer, request_info.request_api_version)?;

        //in every response we want to decrypt the message content
        //we take every record batch content, unwrap and decode it
        //using the relative secure channel
        for response in response.responses.iter_mut() {
            for partition in response.partitions.iter_mut() {
                if let Some(content) = partition.records.take() {
                    let mut content = BytesMut::from(content.as_ref());
                    let mut records = RecordBatchDecoder::decode(&mut content)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    for record in records.iter_mut() {
                        if let Some(record_value) = record.value.take() {
                            let message_wrapper: MessageWrapper =
                                Decoder::new(record_value.as_ref()).decode().map_err(|_| {
                                    InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                })?;

                            let decrypted_content = self
                                .secure_channel_controller
                                .decrypt_content_for(
                                    context,
                                    message_wrapper.secure_channel_identifier,
                                    message_wrapper.content,
                                )
                                .await
                                .map_err(InterceptError::Ockam)?;

                            record.value = Some(decrypted_content.into());
                        }
                    }

                    let mut encoded = BytesMut::new();
                    RecordBatchEncoder::encode(
                        &mut encoded,
                        records.iter(),
                        &RecordEncodeOptions {
                            version: 2,
                            compression: Compression::None,
                        },
                    )
                    .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
                    partition.records = Some(encoded.freeze());
                }
            }
        }

        encode(header, &response, request_info.request_api_version)
    }
}
