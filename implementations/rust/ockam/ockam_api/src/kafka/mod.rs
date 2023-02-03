mod decoder;
mod encoder;
mod inlet_map;
mod portal_listener;
mod portal_worker;
mod protocol_aware;

use ockam_core::Address;
pub(crate) use portal_listener::KafkaPortalListener;

pub const KAFKA_INTERCEPTOR_ADDRESS: &str = "kafka_interceptor";
pub const KAFKA_BOOTSTRAP_ADDRESS: &str = "kafka_bootstrap";

pub fn kafka_outlet_address(broker_id: i32) -> Address {
    Address::from_string(format!("kafka_outlet_{}", broker_id))
}
