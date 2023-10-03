//!This service allows encrypted transparent communication from the kafka producer
//! to the kafka consumer without any modification in the existing application.

mod inlet_controller;
mod integration_test;
mod length_delimited;
mod outlet_controller;
mod outlet_service;
mod portal_listener;
mod portal_worker;
mod protocol_aware;
mod secure_channel_map;

pub(crate) use inlet_controller::KafkaInletController;
use ockam_core::Address;
pub(crate) use outlet_service::prefix_relay::PrefixRelayService;
pub(crate) use outlet_service::OutletManagerService;
pub(crate) use portal_listener::KafkaPortalListener;
pub(crate) use secure_channel_map::ConsumerNodeAddr;
pub(crate) use secure_channel_map::KafkaSecureChannelControllerImpl;

pub const KAFKA_OUTLET_CONSUMERS: &str = "kafka_consumers";
pub const KAFKA_OUTLET_INTERCEPTOR_ADDRESS: &str = "kafka_interceptor";
pub const KAFKA_OUTLET_BOOTSTRAP_ADDRESS: &str = "kafka_bootstrap";

pub fn kafka_outlet_address(broker_id: i32) -> Address {
    format!("kafka_outlet_{}", broker_id).into()
}
