use bytes::BytesMut;
use futures::TryFutureExt;
use kafka_protocol::messages::{
    ApiKey, FetchResponse, FindCoordinatorResponse, MetadataResponse, ProduceRequest,
    RequestHeader, ResponseHeader, TopicName,
};
use kafka_protocol::protocol::buf::ByteBuf;
use kafka_protocol::protocol::{Decodable, Encodable, StrBytes};
use kafka_protocol::records::{
    Compression, RecordBatchDecoder, RecordBatchEncoder, RecordEncodeOptions,
};
use minicbor::{Decode, Decoder, Encode, Encoder};
use tracing::info;

use ockam_core::compat::collections::HashMap;
use ockam_core::compat::fmt::Debug;
use ockam_core::compat::io::{Error, ErrorKind};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, Mutex};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Address, AsyncTryClone, CowStr};
use ockam_identity::api::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse,
};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, SecureChannelRegistryEntry, TrustEveryonePolicy,
};
use ockam_node::Context;

use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::portal_worker::InterceptError;

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

#[derive(AsyncTryClone)]
pub(crate) struct ProtocolState {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
}

use crate::kafka::secure_channel_map::{KafkaSecureChannelController, UniqueSecureChannelId};
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

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
    ) -> ProtocolState {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
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
            info!(
                "request: length: {}, version {:?}, api {:?}",
                buffer.len(),
                header.request_api_version,
                api_key
            );

            match api_key {
                ApiKey::ProduceKey => {
                    let mut request: ProduceRequest =
                        Self::decode(&mut buffer, header.request_api_version)?;

                    //the content can be set in multiple topics and partitions in a single message
                    //for each we wrap the content and add the secure channel identifier of
                    //the encrypted content
                    for (topic_name, topic) in request.topic_data.iter_mut() {
                        for data in &mut topic.partition_data {
                            if let Some(content) = data.records.take() {
                                let mut content = BytesMut::from(content.as_ref());
                                let mut records = RecordBatchDecoder::decode(&mut content)
                                    .map_err(|_| {
                                        InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                    })?;

                                for record in records.iter_mut() {
                                    if let Some(record_value) = record.value.take() {
                                        let (unique_id, encrypted_content) = self
                                            .secure_channel_controller
                                            .encrypt_content_for(
                                                context,
                                                topic_name,
                                                data.index,
                                                record_value.to_vec(),
                                            )
                                            .map_err(InterceptError::Ockam)
                                            .await?;

                                        //TODO: to target multiple consumers we could duplicate
                                        // the content with a dedicated encryption for each consumer
                                        let wrapper = MessageWrapper {
                                            #[cfg(feature = "tag")]
                                            tag: TypeTag,
                                            secure_channel_identifier: unique_id,
                                            content: encrypted_content,
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
                                .map_err(|_| {
                                    InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                })?;

                                data.records = Some(encoded.freeze());
                            }
                        }
                    }

                    let mut modified_buffer = BytesMut::new();

                    //todo: use common encoder
                    header
                        .encode(&mut modified_buffer, header.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
                    request
                        .encode(&mut modified_buffer, header.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    return Ok(modified_buffer);
                }
                ApiKey::MetadataKey | ApiKey::FindCoordinatorKey | ApiKey::FetchKey => {
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

            match request_info.request_api_key {
                ApiKey::FetchKey => {
                    let mut response: FetchResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

                    for response in response.responses.iter_mut() {
                        for partition in response.partitions.iter_mut() {
                            if let Some(content) = partition.records.take() {
                                let mut content = BytesMut::from(content.as_ref());
                                let mut records = RecordBatchDecoder::decode(&mut content)
                                    .map_err(|_| {
                                        InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                    })?;

                                for record in records.iter_mut() {
                                    if let Some(record_value) = record.value.take() {
                                        let message_wrapper: MessageWrapper = Decoder::new(
                                            record_value.as_ref(),
                                        )
                                        .decode()
                                        .map_err(|_| {
                                            InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                        })?;

                                        let secure_channel_entry = self
                                            .get_secure_channel_worker_for(
                                                message_wrapper.secure_channel_identifier.as_ref(),
                                            )?;

                                        let decrypt_response = context
                                            .send_and_receive(
                                                route![secure_channel_entry
                                                    .decryptor_api_address()
                                                    .clone()],
                                                DecryptionRequest(message_wrapper.content),
                                            )
                                            .await
                                            .map_err(InterceptError::Ockam)?;

                                        let decrypted_content = match decrypt_response {
                                            DecryptionResponse::Ok(p) => p,
                                            DecryptionResponse::Err(cause) => {
                                                error!("cannot decrypt kafka message: closing connection");
                                                return Err(InterceptError::Ockam(cause));
                                            }
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
                                .map_err(|_| {
                                    InterceptError::Io(Error::from(ErrorKind::InvalidData))
                                })?;
                                partition.records = Some(encoded.freeze());
                            }
                        }
                    }

                    return Self::encode_response(
                        header,
                        &response,
                        request_info.request_api_version,
                    );
                }

                ApiKey::FindCoordinatorKey => {
                    let mut response: FindCoordinatorResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

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

                    return Self::encode_response(
                        header,
                        &response,
                        request_info.request_api_version,
                    );
                }

                //for metadata we want to replace broker address and port
                // to dedicated tcp inlet ports
                ApiKey::MetadataKey => {
                    let mut response: MetadataResponse =
                        Self::decode(&mut buffer, request_info.request_api_version)?;

                    info!("metadata response before: {:?}", &response);

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
                    info!("metadata response after: {:?}", &response);

                    return Self::encode_response(
                        header,
                        &response,
                        request_info.request_api_version,
                    );
                }
                _ => {}
            }
        }

        Ok(original)
    }

    fn encode_response<T: Encodable>(
        header: ResponseHeader,
        response: &T,
        api_version: i16,
    ) -> Result<BytesMut, InterceptError> {
        let mut buffer = BytesMut::new();

        header
            .encode(&mut buffer, api_version)
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
        response
            .encode(&mut buffer, api_version)
            .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

        return Ok(buffer);
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
