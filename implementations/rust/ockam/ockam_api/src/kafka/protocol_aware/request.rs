use bytes::{Bytes, BytesMut};
use kafka_protocol::messages::fetch_request::FetchRequest;
use kafka_protocol::messages::produce_request::ProduceRequest;
use kafka_protocol::messages::request_header::RequestHeader;
use kafka_protocol::messages::ApiKey;
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::Decodable;
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::encode::Encoder;
use ockam_node::Context;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use tracing::warn;

use crate::kafka::portal_worker::InterceptError;
use crate::kafka::protocol_aware::utils::{decode_body, encode_request};
use crate::kafka::protocol_aware::{InletInterceptorImpl, MessageWrapper, RequestInfo};

impl InletInterceptorImpl {
    ///Parse request and map request <=> response
    /// fails if anything in the parsing fails to avoid leaking clear text payloads
    pub(crate) async fn intercept_request_impl(
        &self,
        context: &mut Context,
        mut original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        //let's clone the view of the buffer without cloning the content
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

        debug!(
            "request: length: {}, correlation {}, version {}, api {:?}",
            buffer.len(),
            header.correlation_id,
            header.request_api_version,
            api_key
        );

        match api_key {
            ApiKey::ProduceKey => {
                return self
                    .handle_produce_request(context, &mut buffer, &header)
                    .await;
            }
            ApiKey::FetchKey => {
                self.handle_fetch_request(context, &mut buffer, &header)
                    .await?;
            }
            ApiKey::MetadataKey | ApiKey::FindCoordinatorKey => {
                self.request_map.lock().unwrap().insert(
                    header.correlation_id,
                    RequestInfo {
                        request_api_key: api_key,
                        request_api_version: header.request_api_version,
                    },
                );
            }
            //we cannot allow to pass modified hosts with wrong security settings
            //we could somehow map them, but these operations are administrative
            //and should not impact consumer/producer flow
            //this is valid for both LeaderAndIsrKey and UpdateMetadataKey
            ApiKey::LeaderAndIsrKey => {
                warn!("leader and isr key not supported! closing connection");
                return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
            }
            ApiKey::UpdateMetadataKey => {
                warn!("update metadata not supported! closing connection");
                return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
            }
            _ => {}
        }

        Ok(original)
    }

    async fn handle_fetch_request(
        &self,
        context: &mut Context,
        buffer: &mut Bytes,
        header: &RequestHeader,
    ) -> Result<(), InterceptError> {
        let request: FetchRequest = decode_body(buffer, header.request_api_version)?;

        //we intercept every partition interested by the kafka client
        //and create a relay for each
        for topic in &request.topics {
            let topic_id = if header.request_api_version <= 12 {
                topic.topic.0.to_string()
            } else {
                //fetch operation using version >= 13 don't use topic name
                //anymore but uses uuid instead, we built a map using
                //previous Metadata requests
                let topic_id = topic.topic_id.to_string();
                self.uuid_to_name
                    .lock()
                    .unwrap()
                    .get(&topic_id)
                    .cloned()
                    .ok_or_else(|| {
                        warn!("missing map from uuid {topic_id} to name");
                        InterceptError::Io(Error::from(ErrorKind::InvalidData))
                    })?
            };

            let partitions: Vec<i32> = topic
                .partitions
                .iter()
                .map(|partition| partition.partition)
                .collect();

            self.secure_channel_controller
                .start_relays_for(context, &topic_id, partitions)
                .await
                .map_err(InterceptError::Ockam)?
        }

        self.request_map.lock().unwrap().insert(
            header.correlation_id,
            RequestInfo {
                request_api_key: ApiKey::FetchKey,
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

        //the content can be set in multiple topics and partitions in a single message
        //for each we wrap the content and add the secure channel identifier of
        //the encrypted content
        for (topic_name, topic) in request.topic_data.iter_mut() {
            for data in &mut topic.partition_data {
                if let Some(content) = data.records.take() {
                    let mut content = BytesMut::from(content.as_ref());
                    let mut records = RecordBatchDecoder::decode(&mut content)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    for record in records.iter_mut() {
                        if let Some(record_value) = record.value.take() {
                            let encrypted_content = self
                                .secure_channel_controller
                                .encrypt_content_for(
                                    context,
                                    topic_name,
                                    data.index,
                                    record_value.to_vec(),
                                )
                                .await
                                .map_err(InterceptError::Ockam)?;

                            //TODO: to target multiple consumers we could duplicate
                            // the content with a dedicated encryption for each consumer
                            let wrapper = MessageWrapper {
                                consumer_decryptor_address: encrypted_content
                                    .consumer_decryptor_address,
                                content: encrypted_content.content,
                            };

                            let mut write_buffer = Vec::with_capacity(1024);
                            let mut encoder = Encoder::new(&mut write_buffer);
                            encoder.encode(wrapper).map_err(|_err| {
                                InterceptError::Io(Error::from(ErrorKind::InvalidData))
                            })?;

                            record.value = Some(write_buffer.into());
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

                    data.records = Some(encoded.freeze());
                }
            }
        }

        encode_request(
            header,
            &request,
            header.request_api_version,
            ApiKey::ProduceKey,
        )
    }
}
