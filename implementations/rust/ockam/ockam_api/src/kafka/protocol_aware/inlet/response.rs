use crate::kafka::protocol_aware::inlet::InletInterceptorImpl;
use crate::kafka::protocol_aware::utils::{decode_body, encode_response};
use crate::kafka::protocol_aware::{
    InterceptError, KafkaEncryptedContent, KafkaMessageResponseInterceptor, RequestInfo,
};
use crate::kafka::KafkaInletController;
use bytes::{Bytes, BytesMut};
use kafka_protocol::messages::{
    ApiKey, ApiVersionsResponse, FetchResponse, FindCoordinatorResponse, MetadataResponse,
    ResponseHeader,
};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, StrBytes};
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::Decoder;
use ockam_core::async_trait;
use ockam_node::Context;

#[async_trait]
impl KafkaMessageResponseInterceptor for InletInterceptorImpl {
    async fn intercept_response(
        &self,
        context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        // let's clone the view of the buffer without cloning the content
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
            let header = match result {
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

            match request_info.request_api_key {
                ApiKey::ApiVersionsKey => {
                    let response: ApiVersionsResponse =
                        decode_body(&mut buffer, request_info.request_api_version)?;
                    debug!("api versions response header: {:?}", header);
                    debug!("api versions response: {:#?}", response);
                }

                ApiKey::FetchKey => {
                    if self.encrypt_content {
                        return self
                            .handle_fetch_response(context, &mut buffer, &request_info, &header)
                            .await;
                    }
                }

                ApiKey::FindCoordinatorKey => {
                    return self
                        .handle_find_coordinator_response(
                            context,
                            &mut buffer,
                            &self.inlet_map,
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
                            &self.inlet_map,
                            request_info,
                            &header,
                        )
                        .await;
                }
                _ => {}
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

impl InletInterceptorImpl {
    // for metadata we want to replace broker address and port
    // to dedicated tcp inlet ports
    async fn handle_metadata_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        inlet_map: &KafkaInletController,
        request_info: RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: MetadataResponse = decode_body(buffer, request_info.request_api_version)?;

        // we need to keep a map of topic uuid to topic name since fetch
        // operations only use uuid
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
            let inlet_address = inlet_map
                .assert_inlet_for_broker(context, broker_id.0)
                .await?;

            trace!(
                "inlet_address: {} for broker {}",
                &inlet_address,
                broker_id.0
            );

            info.host = StrBytes::from_string(inlet_address.hostname());
            info.port = inlet_address.port() as i32;
        }
        trace!("metadata response after: {:?}", &response);

        encode_response(
            header,
            &response,
            request_info.request_api_version,
            ApiKey::MetadataKey,
        )
    }

    async fn handle_find_coordinator_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        inlet_map: &KafkaInletController,
        request_info: &RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: FindCoordinatorResponse =
            decode_body(buffer, request_info.request_api_version)?;

        // similarly to metadata, we want to express the coordinator using
        // local sidecar ip address
        // the format changed to array since version 4
        if request_info.request_api_version >= 4 {
            for coordinator in response.coordinators.iter_mut() {
                let inlet_address = inlet_map
                    .assert_inlet_for_broker(context, coordinator.node_id.0)
                    .await?;

                coordinator.host = StrBytes::from_string(inlet_address.hostname());
                coordinator.port = inlet_address.port() as i32;
            }
        } else {
            let inlet_address = inlet_map
                .assert_inlet_for_broker(context, response.node_id.0)
                .await?;

            response.host = StrBytes::from_string(inlet_address.hostname());
            response.port = inlet_address.port() as i32;
        }

        encode_response(
            header,
            &response,
            request_info.request_api_version,
            ApiKey::FindCoordinatorKey,
        )
    }

    async fn handle_fetch_response(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        request_info: &RequestInfo,
        header: &ResponseHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut response: FetchResponse = decode_body(buffer, request_info.request_api_version)?;

        // in every response we want to decrypt the message content
        // we take every record batch content, unwrap and decode it
        // using the relative secure channel
        for response in response.responses.iter_mut() {
            for partition in response.partitions.iter_mut() {
                if let Some(content) = partition.records.take() {
                    let mut content = BytesMut::from(content.as_ref());
                    let mut records = RecordBatchDecoder::decode(&mut content)
                        .map_err(|_| InterceptError::InvalidData)?;

                    for record in records.iter_mut() {
                        if let Some(record_value) = record.value.take() {
                            let decrypted_content = if self.encrypted_fields.is_empty() {
                                self.decrypt_whole_record(context, record_value).await?
                            } else {
                                self.decrypt_specific_fields(context, record_value).await?
                            };
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
                    .map_err(|_| InterceptError::InvalidData)?;
                    partition.records = Some(encoded.freeze());
                }
            }
        }

        encode_response(
            header,
            &response,
            request_info.request_api_version,
            ApiKey::FetchKey,
        )
    }

    async fn decrypt_whole_record(
        &self,
        context: &mut Context,
        record_value: Bytes,
    ) -> Result<Vec<u8>, InterceptError> {
        let message_wrapper: KafkaEncryptedContent =
            Decoder::new(record_value.as_ref()).decode()?;

        self.key_exchange_controller
            .decrypt_content(
                context,
                &message_wrapper.consumer_decryptor_address,
                message_wrapper.rekey_counter,
                message_wrapper.content,
            )
            .await
            .map_err(InterceptError::Ockam)
    }

    async fn decrypt_specific_fields(
        &self,
        context: &mut Context,
        record_value: Bytes,
    ) -> Result<Vec<u8>, InterceptError> {
        let mut record_value = serde_json::from_slice::<serde_json::Value>(&record_value)?;

        if let serde_json::Value::Object(map) = &mut record_value {
            for field in &self.encrypted_fields {
                // when the encrypted field is present is expected to be a hex encoded string
                // wrapped by the KafkaEncryptedContent struct
                if let Some(value) = map.get_mut(field) {
                    let encrypted_content = if let serde_json::Value::String(string) = value {
                        hex::decode(string).map_err(|_| "Encrypted is not a valid hex string")?
                    } else {
                        error!("encrypted field is not a hex string");
                        return Err("The encrypted field is not a hex-encoded string".into());
                    };

                    let message_wrapper: KafkaEncryptedContent =
                        Decoder::new(&encrypted_content).decode()?;

                    let decrypted_content = self
                        .key_exchange_controller
                        .decrypt_content(
                            context,
                            &message_wrapper.consumer_decryptor_address,
                            message_wrapper.rekey_counter,
                            message_wrapper.content,
                        )
                        .await
                        .map_err(InterceptError::Ockam)?;

                    *value = serde_json::from_slice(decrypted_content.as_slice())?;
                }
            }
            serde_json::to_vec(&record_value).map_err(|error| {
                error!("cannot serialize decrypted fields");
                error.into()
            })
        } else {
            error!(
                "cannot decrypt specific fields, expected a JSON object but got a different type"
            );
            Err("Only JSON objects are supported in the message".into())
        }
    }
}
