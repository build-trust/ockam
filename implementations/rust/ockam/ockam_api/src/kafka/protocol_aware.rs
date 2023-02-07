use bytes::BytesMut;
use kafka_protocol::messages::{
    ApiKey, FetchRequest, FetchResponse, FindCoordinatorResponse, MetadataResponse, ProduceRequest,
    RequestHeader, ResponseHeader,
};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_core::compat::{
    collections::HashMap,
    fmt::Debug,
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use ockam_core::AsyncTryClone;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_node::Context;
use tracing::info;

use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::portal_worker::InterceptError;
use crate::kafka::secure_channel_map::{KafkaSecureChannelController, UniqueSecureChannelId};

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

///map shared across all kafka workers, since the client might request it
/// only from one connection
pub(super) type TopicUuidMap = Arc<Mutex<HashMap<String, String>>>;

#[derive(AsyncTryClone)]
pub(crate) struct ProtocolState {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    uuid_to_name: TopicUuidMap,
    secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
///Wraps the content within every record batch
struct MessageWrapper {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1652220>,
    #[b(1)] secure_channel_identifier: UniqueSecureChannelId,
    #[b(2)] content: Vec<u8>
}

impl ProtocolState {
    pub(crate) fn new(
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        uuid_to_name: TopicUuidMap,
    ) -> ProtocolState {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            uuid_to_name,
            secure_channel_controller,
        }
    }

    ///Parse request and map request <=> response
    /// fails if anything in the parsing fails to avoid leaking clear text payloads
    pub(crate) async fn intercept_request(
        &self,
        context: &mut Context,
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
            trace!(
                "request: length: {}, correlation {}, version {}, api {:?}",
                buffer.len(),
                header.correlation_id,
                header.request_api_version,
                api_key
            );

            match api_key {
                ApiKey::ProduceKey => {
                    let mut request: ProduceRequest =
                        Self::decode(&mut buffer, header.request_api_version)?;

                    return self
                        .handle_produce_request(context, &header, &mut request)
                        .await;
                }
                ApiKey::FetchKey => {
                    let request: FetchRequest =
                        Self::decode(&mut buffer, header.request_api_version)?;

                    self.handle_fetch_request(context, &header, api_key, &request)
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
        } else {
            warn!("unknown request api: {:?}", header.request_api_key);
            return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
        }

        Ok(original)
    }

    async fn handle_fetch_request(
        &self,
        context: &mut Context,
        header: &RequestHeader,
        api_key: ApiKey,
        request: &FetchRequest,
    ) -> Result<(), InterceptError> {
        //we intercept every partition interested by the kafka client
        //and create a forwarder for each
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
                .start_forwarders_for(context, &topic_id, partitions)
                .await
                .map_err(InterceptError::Ockam)?
        }

        self.request_map.lock().unwrap().insert(
            header.correlation_id,
            RequestInfo {
                request_api_key: api_key,
                request_api_version: header.request_api_version,
            },
        );
        Ok(())
    }

    async fn handle_produce_request(
        &self,
        context: &mut Context,
        header: &RequestHeader,
        request: &mut ProduceRequest,
    ) -> Result<BytesMut, InterceptError> {
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
                                #[cfg(feature = "tag")]
                                tag: TypeTag,
                                secure_channel_identifier: encrypted_content.secure_channel_id,
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

        Self::encode(header, request, header.request_api_version)
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
                "response: length: {}, correlation {}, version {}, api {:?}",
                buffer.len(),
                correlation_id,
                request_info.request_api_version,
                request_info.request_api_key
            );

            match request_info.request_api_key {
                ApiKey::FetchKey => {
                    let mut response: FetchResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

                    return self
                        .handle_fetch_response(context, &request_info, &header, &mut response)
                        .await;
                }

                ApiKey::FindCoordinatorKey => {
                    let mut response: FindCoordinatorResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

                    return Self::handle_find_coordinator_response(
                        context,
                        inlet_map,
                        &request_info,
                        &header,
                        &mut response,
                    )
                    .await;
                }

                //for metadata we want to replace broker address and port
                // to dedicated tcp inlet ports
                ApiKey::MetadataKey => {
                    let mut response: MetadataResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

                    return self
                        .handle_metadata_response(
                            context,
                            inlet_map,
                            request_info,
                            &header,
                            &mut response,
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

    async fn handle_metadata_response(
        &self,
        context: &mut Context,
        inlet_map: &KafkaInletMap,
        request_info: RequestInfo,
        header: &ResponseHeader,
        response: &mut MetadataResponse,
    ) -> Result<BytesMut, InterceptError> {
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
            info.host = Self::string_to_str_bytes(ip_address);
            info.port = inlet_address.port() as i32;
        }
        trace!("metadata response after: {:?}", &response);

        Self::encode(header, response, request_info.request_api_version)
    }

    async fn handle_find_coordinator_response(
        context: &mut Context,
        inlet_map: &KafkaInletMap,
        request_info: &RequestInfo,
        header: &ResponseHeader,
        mut response: &mut FindCoordinatorResponse,
    ) -> Result<BytesMut, InterceptError> {
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
                coordinator.host = Self::string_to_str_bytes(ip_address);
                coordinator.port = inlet_address.port() as i32;
            }
        } else {
            let inlet_address: SocketAddr = inlet_map
                .assert_inlet_for_broker(context, response.node_id.0)
                .await
                .map_err(InterceptError::Ockam)?;

            let ip_address = inlet_address.ip().to_string();
            response.host = Self::string_to_str_bytes(ip_address);
            response.port = inlet_address.port() as i32;
        }

        Self::encode(header, response, request_info.request_api_version)
    }

    async fn handle_fetch_response(
        &self,
        context: &mut Context,
        request_info: &RequestInfo,
        header: &ResponseHeader,
        response: &mut FetchResponse,
    ) -> Result<BytesMut, InterceptError> {
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

        Self::encode(header, response, request_info.request_api_version)
    }

    fn encode<H: Encodable, T: Encodable>(
        header: &H,
        body: &T,
        api_version: i16,
    ) -> Result<BytesMut, InterceptError> {
        let mut buffer = BytesMut::new();

        header
            .encode(&mut buffer, api_version)
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
        body.encode(&mut buffer, api_version)
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        Ok(buffer)
    }

    fn string_to_str_bytes(ip_address: String) -> StrBytes {
        //TryFrom is broken, ugly but effective
        unsafe { StrBytes::from_utf8_unchecked(bytes::Bytes::from(ip_address)) }
    }

    fn decode<T, B>(buffer: &mut B, api_version: i16) -> Result<T, InterceptError>
    where
        T: Decodable,
        B: ByteBuf,
    {
        let response = match T::decode(buffer, api_version) {
            Ok(response) => response,
            Err(_) => {
                warn!("cannot decode kafka message");
                return Err(InterceptError::Io(Error::from(ErrorKind::InvalidData)));
            }
        };
        Ok(response)
    }
}
