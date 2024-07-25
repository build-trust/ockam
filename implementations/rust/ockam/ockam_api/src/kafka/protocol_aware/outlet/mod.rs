use crate::kafka::protocol_aware::{
    CorrelationId, KafkaMessageInterceptor, KafkaMessageInterceptorWrapper, RequestInfo,
    MAX_KAFKA_MESSAGE_SIZE,
};
use crate::kafka::KafkaOutletController;
use ockam_core::compat::collections::HashMap;
use ockam_core::flow_control::FlowControlId;
use ockam_transport_tcp::{PortalInterceptor, PortalInterceptorFactory};
use std::sync::{Arc, Mutex};

mod request;
mod response;

pub(crate) struct KafkaOutletInterceptorFactory {
    outlet_controller: KafkaOutletController,
    spawner_flow_control_id: FlowControlId,
}

impl KafkaOutletInterceptorFactory {
    pub(crate) fn new(
        outlet_controller: KafkaOutletController,
        spawner_flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            outlet_controller,
            spawner_flow_control_id,
        }
    }
}

impl PortalInterceptorFactory for KafkaOutletInterceptorFactory {
    fn create(&self) -> Arc<dyn PortalInterceptor> {
        Arc::new(KafkaMessageInterceptorWrapper::new(
            Arc::new(OutletInterceptorImpl::new(
                self.outlet_controller.clone(),
                self.spawner_flow_control_id.clone(),
            )),
            MAX_KAFKA_MESSAGE_SIZE,
        ))
    }
}

/// Intercepts responses of type `Metadata` to extract the list of brokers
/// then creates an outlet for each of them through [`KafkaOutletController`]
#[derive(Clone)]
pub(crate) struct OutletInterceptorImpl {
    request_map: Arc<Mutex<HashMap<CorrelationId, RequestInfo>>>,
    outlet_controller: KafkaOutletController,
    flow_control_id: FlowControlId,
}

impl OutletInterceptorImpl {
    pub(crate) fn new(
        outlet_controller: KafkaOutletController,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            request_map: Arc::new(Mutex::new(HashMap::new())),
            outlet_controller,
            flow_control_id,
        }
    }
}

impl KafkaMessageInterceptor for OutletInterceptorImpl {}
