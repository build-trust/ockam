use bytes::BytesMut;
use kafka_protocol::messages::ApiKey;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use ockam_core::{async_trait, Address, Error};
use ockam_node::Context;

pub(crate) mod outlet;
mod tests;

pub(crate) mod inlet;
mod length_delimited;
pub(super) mod utils;

use crate::kafka::protocol_aware::length_delimited::{length_encode, KafkaMessageDecoder};
use ockam_core::errcode::{Kind, Origin};
use ockam_transport_tcp::{Direction, PortalInterceptor};

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

/// Map shared across all kafka workers, since the client might request it
/// only from one connection
pub(super) type TopicUuidMap = Arc<Mutex<HashMap<String, String>>>;

#[async_trait]
pub(crate) trait KafkaMessageInterceptorRequest: Send + Sync + 'static {
    async fn intercept_request(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError>;
}

#[async_trait]
pub(crate) trait KafkaMessageInterceptorResponse: Send + Sync + 'static {
    async fn intercept_response(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError>;
}

#[async_trait]
pub(crate) trait KafkaMessageInterceptor:
    KafkaMessageInterceptorRequest + KafkaMessageInterceptorResponse + Send + Sync + 'static
{
}

pub struct KafkaMessageInterceptorWrapper {
    decoder_from_inlet: Arc<Mutex<KafkaMessageDecoder>>,
    decoder_from_outlet: Arc<Mutex<KafkaMessageDecoder>>,
    message_interceptor: Arc<dyn KafkaMessageInterceptor>,
    max_message_size: u32,
}

/// Converts a generic interceptor trait into kafka specific interceptor
impl KafkaMessageInterceptorWrapper {
    pub fn new(
        message_interceptor: Arc<dyn KafkaMessageInterceptor>,
        max_message_size: u32,
    ) -> Self {
        Self {
            decoder_from_inlet: Arc::new(Mutex::new(KafkaMessageDecoder::new())),
            decoder_from_outlet: Arc::new(Mutex::new(KafkaMessageDecoder::new())),
            message_interceptor,
            max_message_size,
        }
    }
}

#[async_trait]
impl PortalInterceptor for KafkaMessageInterceptorWrapper {
    async fn intercept(
        &self,
        context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>> {
        let mut encoded_buffer: Option<BytesMut> = None;

        let messages = {
            let decoder = match direction {
                Direction::FromOutletToInlet => &self.decoder_from_outlet,
                Direction::FromInletToOutlet => &self.decoder_from_inlet,
            };

            let mut guard = decoder.lock().unwrap();
            guard.extract_complete_messages(BytesMut::from(buffer), self.max_message_size)?
        };

        for complete_kafka_message in messages {
            let transformed_message = match direction {
                Direction::FromInletToOutlet => {
                    self.message_interceptor
                        .intercept_request(context, complete_kafka_message)
                        .await
                }
                Direction::FromOutletToInlet => {
                    self.message_interceptor
                        .intercept_response(context, complete_kafka_message)
                        .await
                }
            }
            .map_err(|error| match error {
                InterceptError::Io(cause) => Error::new(Origin::Transport, Kind::Io, cause),
                InterceptError::Ockam(error) => error,
            })?;

            // avoid copying the first message
            if let Some(encoded_buffer) = encoded_buffer.as_mut() {
                encoded_buffer.extend_from_slice(length_encode(transformed_message)?.as_ref());
            } else {
                encoded_buffer = Some(length_encode(transformed_message)?)
            }
        }

        Ok(encoded_buffer.map(|buffer| buffer.freeze().to_vec()))
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
/// Wraps the content within every record batch
struct MessageWrapper {
    #[n(0)] consumer_decryptor_address: Address,
    #[n(1)] content: Vec<u8>
}

/// By default, kafka supports up to 1MB messages. 16MB is the maximum suggested
pub(crate) const MAX_KAFKA_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

// internal error to return both io and ockam errors
#[derive(Debug)]
pub(crate) enum InterceptError {
    Io(ockam_core::compat::io::Error),
    Ockam(ockam_core::Error),
}
