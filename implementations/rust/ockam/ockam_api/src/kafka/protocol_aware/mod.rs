use crate::kafka::portal_worker::InterceptError;
use crate::kafka::secure_channel_map::KafkaSecureChannelController;
use crate::kafka::KafkaInletController;
use bytes::BytesMut;
use kafka_protocol::messages::ApiKey;
use minicbor::{Decode, Encode};
use ockam_core::compat::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use ockam_core::{async_trait, Address};
use ockam_node::Context;

mod metadata_interceptor;
mod request;
mod response;
mod tests;

pub(super) mod utils;
pub(crate) use metadata_interceptor::OutletInterceptorImpl;

#[derive(Clone, Debug)]
struct RequestInfo {
    pub request_api_key: ApiKey,
    pub request_api_version: i16,
}

type CorrelationId = i32;

/// map shared across all kafka workers, since the client might request it
/// only from one connection
pub(super) type TopicUuidMap = Arc<Mutex<HashMap<String, String>>>;

#[async_trait]
pub(crate) trait KafkaMessageInterceptor: Send + Sync + 'static {
    async fn intercept_request(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError>;

    async fn intercept_response(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError>;
}

#[derive(Clone)]
pub(crate) struct InletInterceptorImpl {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    uuid_to_name: TopicUuidMap,
    secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
    inlet_map: KafkaInletController,
}

#[async_trait]
impl KafkaMessageInterceptor for InletInterceptorImpl {
    async fn intercept_request(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        self.intercept_request_impl(context, original).await
    }

    async fn intercept_response(
        &self,
        context: &mut Context,
        original: BytesMut,
    ) -> Result<BytesMut, InterceptError> {
        self.intercept_response_impl(context, original).await
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
///Wraps the content within every record batch
struct MessageWrapper {
    #[n(1)] consumer_decryptor_address: Address,
    #[n(2)] content: Vec<u8>
}

impl InletInterceptorImpl {
    pub(crate) fn new(
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        uuid_to_name: TopicUuidMap,
        inlet_map: KafkaInletController,
    ) -> InletInterceptorImpl {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            uuid_to_name,
            secure_channel_controller,
            inlet_map,
        }
    }
}
