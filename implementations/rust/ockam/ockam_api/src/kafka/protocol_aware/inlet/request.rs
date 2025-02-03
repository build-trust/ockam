use crate::kafka::protocol_aware::inlet::InletInterceptorImpl;
use crate::kafka::protocol_aware::utils::{decode_body, encode_request};
use crate::kafka::protocol_aware::RequestInfo;
use crate::kafka::protocol_aware::{InterceptError, KafkaMessageRequestInterceptor};
use bytes::{Bytes, BytesMut};
use kafka_protocol::messages::fetch_request::FetchRequest;
use kafka_protocol::messages::produce_request::{PartitionProduceData, ProduceRequest};
use kafka_protocol::messages::request_header::RequestHeader;
use kafka_protocol::messages::{ApiKey, ApiVersionsRequest, TopicName};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Message};
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::encode::Encoder;
use ockam_core::async_trait;
use ockam_node::Context;
use std::convert::TryFrom;
use tracing::warn;

#[async_trait]
impl KafkaMessageRequestInterceptor for InletInterceptorImpl {
    /// Parse request, map request <=> response, and modify some requests.
    /// Returns an error if the parsing fails to avoid leaking clear text payloads
    async fn intercept_request(
        &self,
        context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        // let's clone the view of the buffer without cloning the content
        let mut buffer = original.peek_bytes(0..original.len());

        // Inside the request we can find the api key (kind of request), the protocol version of the request,
        // and the identifier of the request.
        // The request identifier, called correlation id, that we can use to map the request
        // to the response.
        // for more information see:
        // https://cwiki.apache.org/confluence/display/KAFKA/A+Guide+To+The+Kafka+Protocol#AGuideToTheKafkaProtocol-Requests

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

        match api_key {
            ApiKey::ApiVersions => {
                debug!("api versions request: {:?}", header);
                return self.handle_api_version_request(&mut buffer, &header).await;
            }

            ApiKey::Produce => {
                if self.encrypt_content {
                    return self
                        .handle_produce_request(context, &mut buffer, &header)
                        .await;
                }
            }
            ApiKey::Fetch => {
                self.handle_fetch_request(context, &mut buffer, &header)
                    .await?;
            }
            ApiKey::Metadata | ApiKey::FindCoordinator => {
                self.request_map.lock().unwrap().insert(
                    header.correlation_id,
                    RequestInfo {
                        request_api_key: api_key,
                        request_api_version: header.request_api_version,
                    },
                );
            }
            // we cannot allow passing modified hosts with wrong security settings
            // we could somehow map them, but these operations are administrative
            // and should not impact consumer/producer flow
            // this is valid for both LeaderAndIsrKey and UpdateMetadataKey
            ApiKey::LeaderAndIsr => {
                warn!("leader and isr key not supported! closing connection");
                return Err(InterceptError::InvalidData);
            }
            ApiKey::UpdateMetadata => {
                warn!("update metadata not supported! closing connection");
                return Err(InterceptError::InvalidData);
            }
            _ => {}
        }

        Ok(original)
    }
}

impl InletInterceptorImpl {
    async fn handle_api_version_request(
        &self,
        buffer: &mut Bytes,
        header: &RequestHeader,
    ) -> Result<BytesMut, InterceptError> {
        let request: ApiVersionsRequest = decode_body(buffer, header.request_api_version)?;
        const MAX_SUPPORTED_VERSION: i16 = ApiVersionsRequest::VERSIONS.max;

        let request_api_version = if header.request_api_version > MAX_SUPPORTED_VERSION {
            warn!("api versions request with version > {MAX_SUPPORTED_VERSION} not supported, downgrading request to {MAX_SUPPORTED_VERSION}");
            MAX_SUPPORTED_VERSION
        } else {
            header.request_api_version
        };

        self.request_map.lock().unwrap().insert(
            header.correlation_id,
            RequestInfo {
                request_api_key: ApiKey::ApiVersions,
                request_api_version,
            },
        );

        let mut header = header.clone();
        header.request_api_version = request_api_version;

        encode_request(&header, &request, request_api_version, ApiKey::ApiVersions)
    }

    async fn handle_fetch_request(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        header: &RequestHeader,
    ) -> Result<(), InterceptError> {
        let request: FetchRequest = decode_body(buffer, header.request_api_version)?;

        // we intercept every partition interested in the kafka client
        // and create a relay for each
        for topic in &request.topics {
            let topic_id = if header.request_api_version <= 12 {
                topic.topic.0.to_string()
            } else {
                // fetch operation using version >= 13 don't use topic name
                // anymore but uses uuid instead, we built a map using
                // previous Metadata requests
                let topic_id = topic.topic_id.to_string();
                self.uuid_to_name
                    .lock()
                    .unwrap()
                    .get(&topic_id)
                    .cloned()
                    .ok_or_else(|| {
                        warn!("missing map from uuid {topic_id} to name");
                        InterceptError::InvalidData
                    })?
            };

            let partitions: Vec<i32> = topic
                .partitions
                .iter()
                .map(|partition| partition.partition)
                .collect();

            self.key_exchange_controller
                .publish_consumer(context, &topic_id, partitions)
                .await
                .map_err(InterceptError::Ockam)?
        }

        self.request_map.lock().unwrap().insert(
            header.correlation_id,
            RequestInfo {
                request_api_key: ApiKey::Fetch,
                request_api_version: header.request_api_version,
            },
        );
        Ok(())
    }

    async fn handle_produce_request(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        header: &RequestHeader,
    ) -> Result<BytesMut, InterceptError> {
        let mut request: ProduceRequest = decode_body(buffer, header.request_api_version)?;

        // the content can be set in multiple topics and partitions in a single message
        // for each we wrap the content and add the secure channel identifier of
        // the encrypted content
        for topic in request.topic_data.iter_mut() {
            for data in &mut topic.partition_data {
                if let Some(content) = data.records.take() {
                    let mut content = BytesMut::from(content.as_ref());
                    let mut records = RecordBatchDecoder::decode::<
                        BytesMut,
                        fn(&mut Bytes, Compression) -> Result<BytesMut, _>,
                    >(&mut content)
                    .map_err(|_| InterceptError::InvalidData)?;

                    for record in records.iter_mut() {
                        if let Some(record_value) = record.value.take() {
                            let buffer = if !self.encrypted_fields.is_empty() {
                                // if we encrypt only specific fields, we assume the record must be
                                // valid JSON map
                                self.encrypt_specific_fields(
                                    context,
                                    &topic.name,
                                    data,
                                    &record_value,
                                )
                                .await?
                            } else {
                                self.encrypt_whole_record(context, &topic.name, data, record_value)
                                    .await?
                            };
                            record.value = Some(buffer.into());
                        }
                    }

                    let mut encoded = BytesMut::new();
                    RecordBatchEncoder::encode::<
                        BytesMut,
                        std::slice::Iter<'_, kafka_protocol::records::Record>,
                        fn(&mut BytesMut, &mut BytesMut, Compression) -> Result<(), _>,
                    >(
                        &mut encoded,
                        records.iter(),
                        &RecordEncodeOptions {
                            version: 2,
                            compression: Compression::None,
                        },
                    )
                    .map_err(|_| InterceptError::InvalidData)?;

                    data.records = Some(encoded.freeze());
                }
            }
        }

        encode_request(
            header,
            &request,
            header.request_api_version,
            ApiKey::Produce,
        )
    }

    async fn encrypt_whole_record(
        &self,
        context: &mut Context,
        topic_name: &TopicName,
        data: &mut PartitionProduceData,
        record_value: Bytes,
    ) -> Result<Vec<u8>, InterceptError> {
        let encrypted_content = self
            .key_exchange_controller
            .encrypt_content(context, topic_name, data.index, record_value.to_vec())
            .await
            .map_err(InterceptError::Ockam)?;

        let mut write_buffer = Vec::with_capacity(1024);
        let mut encoder = Encoder::new(&mut write_buffer);
        encoder
            .encode(encrypted_content)
            .map_err(|_err| InterceptError::InvalidData)?;

        Ok(write_buffer)
    }

    async fn encrypt_specific_fields(
        &self,
        context: &mut Context,
        topic_name: &TopicName,
        data: &mut PartitionProduceData,
        record_value: &Bytes,
    ) -> Result<Vec<u8>, InterceptError> {
        let mut record_value = serde_json::from_slice::<serde_json::Value>(record_value)?;

        if let serde_json::Value::Object(map) = &mut record_value {
            for field in &self.encrypted_fields {
                if let Some(value) = map.get_mut(field) {
                    let encrypted_content = self
                        .key_exchange_controller
                        .encrypt_content(
                            context,
                            topic_name,
                            data.index,
                            serde_json::to_vec(value).map_err(|_| InterceptError::InvalidData)?,
                        )
                        .await
                        .map_err(InterceptError::Ockam)?;

                    let mut write_buffer = Vec::with_capacity(1024);
                    let mut encoder = Encoder::new(&mut write_buffer);
                    encoder
                        .encode(encrypted_content)
                        .map_err(|_| InterceptError::InvalidData)?;
                    *value = serde_json::Value::String(hex::encode(&write_buffer));
                }
            }
        } else {
            warn!("only JSON objects are supported for field encryption");
            return Err("Only JSON objects are supported".into());
        }

        Ok(record_value.to_string().as_bytes().to_vec())
    }
}
