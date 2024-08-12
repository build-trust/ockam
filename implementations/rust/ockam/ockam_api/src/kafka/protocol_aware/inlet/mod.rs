use crate::kafka::key_exchange::controller::KafkaKeyExchangeController;
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

#[derive(Clone)]
pub(crate) struct InletInterceptorImpl {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    uuid_to_name: TopicUuidMap,
    key_exchange_controller: KafkaKeyExchangeController,
    inlet_map: KafkaInletController,
    encrypt_content: bool,
}

#[async_trait]
impl KafkaMessageInterceptor for InletInterceptorImpl {}

impl InletInterceptorImpl {
    pub(crate) fn new(
        key_exchange_controller: KafkaKeyExchangeController,
        uuid_to_name: TopicUuidMap,
        inlet_map: KafkaInletController,
        encrypt_content: bool,
    ) -> InletInterceptorImpl {
        Self {
            request_map: Arc::new(Mutex::new(Default::default())),
            uuid_to_name,
            key_exchange_controller,
            inlet_map,
            encrypt_content,
        }
    }
}

pub(crate) struct KafkaInletInterceptorFactory {
    secure_channel_controller: KafkaKeyExchangeController,
    uuid_to_name: TopicUuidMap,
    inlet_map: KafkaInletController,
    encrypt_content: bool,
}

impl KafkaInletInterceptorFactory {
    pub(crate) fn new(
        secure_channel_controller: KafkaKeyExchangeController,
        inlet_map: KafkaInletController,
        encrypt_content: bool,
    ) -> Self {
        Self {
            secure_channel_controller,
            uuid_to_name: Default::default(),
            inlet_map,
            encrypt_content,
        }
    }
}

impl PortalInterceptorFactory for KafkaInletInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(KafkaMessageInterceptorWrapper::new(
            Arc::new(InletInterceptorImpl::new(
                self.secure_channel_controller.clone(),
                self.uuid_to_name.clone(),
                self.inlet_map.clone(),
                self.encrypt_content,
            )),
            MAX_KAFKA_MESSAGE_SIZE,
        ))
    }
}
