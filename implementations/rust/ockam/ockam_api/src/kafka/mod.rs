//!This service allows encrypted transparent communication from the kafka producer
//! to the kafka consumer without any modification in the existing application.

mod inlet_controller;
pub(crate) mod key_exchange;
mod outlet_controller;
pub(crate) mod protocol_aware;
#[cfg(test)]
mod tests;

pub(crate) use inlet_controller::KafkaInletController;
pub use key_exchange::{ConsumerPublishing, ConsumerResolution};
use ockam::identity::Identifier;
use ockam_abac::expr::{eq, or, str};
use ockam_abac::{subject_has_credential_policy_expression, subject_identifier_attribute, Expr};
use ockam_core::Address;
pub(crate) use outlet_controller::KafkaOutletController;

pub const KAFKA_OUTLET_INTERCEPTOR_ADDRESS: &str = "kafka_interceptor";
pub const KAFKA_OUTLET_BOOTSTRAP_ADDRESS: &str = "kafka_bootstrap";

pub fn kafka_outlet_address(broker_id: i32) -> Address {
    format!("kafka_outlet_{}", broker_id).into()
}

pub fn kafka_policy_expression(project_identifier: &Identifier) -> Expr {
    or([
        eq([
            subject_identifier_attribute(),
            str(project_identifier.to_string()),
        ]),
        subject_has_credential_policy_expression(),
    ])
}
