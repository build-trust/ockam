use bytes::BytesMut;
use kafka_protocol::messages::{
    ApiKey, FetchResponse, MetadataResponse, ProduceRequest, RequestHeader, ResponseHeader,
    TopicName,
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
pub(crate) struct ProtocolState<V: IdentityVault, S: AuthenticatedStorage> {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    identity: Identity<V, S>,
    current_secure_channel_address: Arc<Mutex<Option<Address>>>,
}

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
struct MessageWrapper<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1652220>,
    #[b(1)] secure_channel_identifier: CowStr<'a>,
    #[b(2)] content: Vec<u8>
}

impl<V: IdentityVault, S: AuthenticatedStorage> ProtocolState<V, S> {
    pub(crate) fn new(identity: Identity<V, S>) -> ProtocolState<V, S> {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            current_secure_channel_address: Arc::new(Mutex::new(None)),
            identity,
        }
    }

    async fn assert_secure_channel_worker_for(
        &self,
        _topic_name: &TopicName,
        _partition_id: i32,
    ) -> Result<SecureChannelRegistryEntry, InterceptError> {
        //here we should have the orchestrator address
        // and expect forwarders to be present in the orchestrator
        // with a format similar to "kafka_consumer_forwarder_{partition}_{topic_name}"

        //for this iteration we will expect to find "kafka_consumer_secure_channel" _locally_

        //either we use an async lock or we just assume no other thread will actually attempt
        //to establish the connection: currently we are doing the latter with a unlocked check
        let current_address = self
            .current_secure_channel_address
            .lock()
            .unwrap()
            .as_ref()
            .cloned();

        let secure_channel_address: Address = {
            if let Some(secure_channel_address) = current_address {
                secure_channel_address
            } else {
                let secure_channel_address = self
                    .identity
                    .create_secure_channel(
                        route!["kafka_consumer_secure_channel"],
                        TrustEveryonePolicy,
                    )
                    .await
                    .map_err(InterceptError::Ockam)?;

                *self.current_secure_channel_address.lock().unwrap() =
                    Some(secure_channel_address.clone());

                secure_channel_address
            }
        };

        self.identity
            .secure_channel_registry()
            .get_channel_by_encryptor_address(&secure_channel_address)
            .ok_or_else(|| {
                InterceptError::Ockam(ockam_core::Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    "secure channel down",
                ))
            })
    }

    fn get_secure_channel_worker_for(
        &self,
        other_party_identifier: &str,
    ) -> Result<SecureChannelRegistryEntry, InterceptError> {
        let identifier = IdentityIdentifier::from_key_id(other_party_identifier);
        self.identity
            .secure_channel_registry()
            .get_channel_list()
            .iter()
            .find(|entry| entry.their_id() == &identifier && !entry.is_initiator())
            .cloned()
            .ok_or_else(|| {
                InterceptError::Ockam(ockam_core::Error::new(
                    Origin::Channel,
                    Kind::Unknown,
                    "secure channel down",
                ))
            })
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
                                        let secure_channel_entry = self
                                            .assert_secure_channel_worker_for(
                                                topic_name, data.index,
                                            )
                                            .await?;

                                        let encryption_response: EncryptionResponse = context
                                            .send_and_receive(
                                                route![secure_channel_entry
                                                    .encryptor_api_address()
                                                    .clone()],
                                                EncryptionRequest(record_value.to_vec()),
                                            )
                                            .await
                                            .map_err(InterceptError::Ockam)?;

                                        let encrypted_content = match encryption_response {
                                            EncryptionResponse::Ok(p) => p,
                                            EncryptionResponse::Err(cause) => {
                                                warn!("cannot encrypt kafka message");
                                                return Err(InterceptError::Ockam(cause));
                                            }
                                        };

                                        //TODO: to target multiple consumers we could duplicate
                                        // the content with a dedicated encryption for each consumer
                                        let wrapper = MessageWrapper {
                                            #[cfg(feature = "tag")]
                                            tag: TypeTag,
                                            secure_channel_identifier: CowStr::from(
                                                //TODO: to be more robust switch to a secure channel
                                                // identifier instead of using an identity based identifier
                                                secure_channel_entry.my_id().key_id(),
                                            ),
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

                    header
                        .encode(&mut modified_buffer, header.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
                    request
                        .encode(&mut modified_buffer, header.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    return Ok(modified_buffer);
                }
                ApiKey::MetadataKey | ApiKey::FetchKey => {
                    self.request_map.lock().unwrap().insert(
                        header.correlation_id,
                        RequestInfo {
                            request_api_key: api_key,
                            request_api_version: header.request_api_version,
                        },
                    );
                }
                ApiKey::OffsetFetchKey => {
                    warn!("offset fetch key not supported! closing connection");
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

                    let mut modified_buffer = BytesMut::new();

                    header
                        .encode(&mut modified_buffer, request_info.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;
                    response
                        .encode(&mut modified_buffer, request_info.request_api_version)
                        .map_err(|_| InterceptError::Io(Error::from(ErrorKind::InvalidData)))?;

                    return Ok(modified_buffer);
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
