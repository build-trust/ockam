//!This service allows encrypted transparent communication from the kafka producer
//! to the kafka consumer without any modification in the existing application.

mod inlet_map;
mod integration_test;
mod length_delimited;
mod portal_listener;
mod portal_worker;
mod protocol_aware;
mod secure_channel_map;

pub(crate) use inlet_map::KafkaInletMap;
use ockam_core::Address;
pub(crate) use portal_listener::KafkaPortalListener;
pub(crate) use secure_channel_map::KafkaSecureChannelController;
pub(crate) use secure_channel_map::KafkaSecureChannelControllerImpl;

pub const ORCHESTRATOR_KAFKA_CONSUMERS: &str = "kafka_consumers";
pub const ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS: &str = "kafka_interceptor";
pub const ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS: &str = "kafka_bootstrap";

pub const KAFKA_SECURE_CHANNEL_CONTROLLER_ADDRESS: &str = "kafka_secure_channel_controller";
pub const KAFKA_SECURE_CHANNEL_LISTENER_ADDRESS: &str = "kafka_consumer_secure_channel";

pub fn kafka_outlet_address(broker_id: i32) -> Address {
    Address::from_string(format!("kafka_outlet_{}", broker_id))
}
