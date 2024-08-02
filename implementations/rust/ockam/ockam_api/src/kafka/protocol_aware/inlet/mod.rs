use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
use crate::kafka::key_exchange::KafkaKeyExchangeController;
use crate::kafka::protocol_aware::{
    CorrelationId, KafkaMessageInterceptor, KafkaMessageInterceptorWrapper, RequestInfo,
    TopicUuidMap, MAX_KAFKA_MESSAGE_SIZE,
};
use crate::kafka::KafkaInletController;
use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_transport_tcp::{PortalInterceptor, PortalInterceptorFactory};
use std::sync::{Arc, Mutex};

mod request;
mod response;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub(crate) struct InletInterceptorImpl {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    uuid_to_name: TopicUuidMap,
    key_exchange_controller: Arc<dyn KafkaKeyExchangeController>,
    inlet_map: KafkaInletController,
    encrypt_content: bool,
    encrypted_fields: Vec<String>,
}

#[async_trait]
impl KafkaMessageInterceptor for InletInterceptorImpl {}

impl InletInterceptorImpl {
    pub(crate) fn new(
        key_exchange_controller: Arc<dyn KafkaKeyExchangeController>,
        uuid_to_name: TopicUuidMap,
        inlet_map: KafkaInletController,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
    ) -> InletInterceptorImpl {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            uuid_to_name,
            key_exchange_controller,
            inlet_map,
            encrypt_content,
            encrypted_fields,
        }
    }

    #[cfg(test)]
    pub(crate) fn add_request(
        &self,
        correlation_id: CorrelationId,
        api_key: kafka_protocol::messages::ApiKey,
        api_version: i16,
    ) {
        self.request_map.lock().unwrap().insert(
            correlation_id,
            RequestInfo {
                request_api_key: api_key,
                request_api_version: api_version,
            },
        );
    }
}

pub(crate) struct KafkaInletInterceptorFactory {
    secure_channel_controller: KafkaKeyExchangeControllerImpl,
    uuid_to_name: TopicUuidMap,
    inlet_map: KafkaInletController,
    encrypt_content: bool,
    encrypted_fields: Vec<String>,
}

impl KafkaInletInterceptorFactory {
    pub(crate) fn new(
        secure_channel_controller: KafkaKeyExchangeControllerImpl,
        inlet_map: KafkaInletController,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
    ) -> Self {
        Self {
            secure_channel_controller,
            uuid_to_name: Default::default(),
            inlet_map,
            encrypt_content,
            encrypted_fields,
        }
    }
}

impl PortalInterceptorFactory for KafkaInletInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(KafkaMessageInterceptorWrapper::new(
            Arc::new(InletInterceptorImpl::new(
                Arc::new(self.secure_channel_controller.clone()),
                self.uuid_to_name.clone(),
                self.inlet_map.clone(),
                self.encrypt_content,
                self.encrypted_fields.clone(),
            )),
            MAX_KAFKA_MESSAGE_SIZE,
        ))
    }
}
