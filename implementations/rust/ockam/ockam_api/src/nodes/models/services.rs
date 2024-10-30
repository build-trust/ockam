use crate::colors::{color_primary, color_warn};
use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use crate::output::Output;
use minicbor::{CborLen, Decode, Encode};
use ockam_abac::PolicyExpression;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;
use ockam_transport_core::HostnamePort;
use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartServiceRequest<T> {
    #[n(1)] addr: String,
    #[n(2)] req: T,
}

impl<T> StartServiceRequest<T> {
    pub fn new<S: Into<String>>(req: T, addr: S) -> Self {
        Self {
            addr: addr.into(),
            req,
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }

    pub fn request(&self) -> &T {
        &self.req
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteServiceRequest {
    #[n(1)] addr: String,
}

impl DeleteServiceRequest {
    pub fn new<S: Into<String>>(addr: S) -> Self {
        Self { addr: addr.into() }
    }

    pub fn address(&self) -> Address {
        Address::from(self.addr.clone())
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaOutletRequest {
    #[n(1)] bootstrap_server_addr: HostnamePort,
    #[n(2)] tls: bool,
    #[n(3)] policy_expression: Option<PolicyExpression>,
}

impl StartKafkaOutletRequest {
    pub fn new(
        bootstrap_server_addr: HostnamePort,
        tls: bool,
        policy_expression: Option<PolicyExpression>,
    ) -> Self {
        Self {
            bootstrap_server_addr,
            tls,
            policy_expression,
        }
    }

    pub fn bootstrap_server_addr(&self) -> HostnamePort {
        self.bootstrap_server_addr.clone()
    }

    pub fn tls(&self) -> bool {
        self.tls
    }

    pub fn policy_expression(&self) -> Option<PolicyExpression> {
        self.policy_expression.clone()
    }
}

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaInletRequest {
    #[n(1)] bind_address: HostnamePort,
    #[n(2)] brokers_port_range: (u16, u16),
    #[n(3)] kafka_outlet_route: MultiAddr,
    #[n(4)] encrypt_content: bool,
    #[n(5)] consumer_resolution: ConsumerResolution,
    #[n(6)] consumer_publishing: ConsumerPublishing,
    #[n(7)] inlet_policy_expression: Option<PolicyExpression>,
    #[n(8)] consumer_policy_expression: Option<PolicyExpression>,
    #[n(9)] producer_policy_expression: Option<PolicyExpression>,
    #[n(10)] encrypted_fields: Vec<String>,
}

impl StartKafkaInletRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bind_address: HostnamePort,
        brokers_port_range: impl Into<(u16, u16)>,
        kafka_outlet_route: MultiAddr,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        inlet_policy_expression: Option<PolicyExpression>,
        consumer_policy_expression: Option<PolicyExpression>,
        producer_policy_expression: Option<PolicyExpression>,
    ) -> Self {
        Self {
            bind_address,
            brokers_port_range: brokers_port_range.into(),
            kafka_outlet_route,
            encrypt_content,
            consumer_resolution,
            consumer_publishing,
            inlet_policy_expression,
            consumer_policy_expression,
            producer_policy_expression,
            encrypted_fields,
        }
    }

    pub fn bind_address(&self) -> HostnamePort {
        self.bind_address.clone()
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> MultiAddr {
        self.kafka_outlet_route.clone()
    }

    pub fn encrypt_content(&self) -> bool {
        self.encrypt_content
    }

    pub fn encrypted_fields(&self) -> Vec<String> {
        self.encrypted_fields.clone()
    }

    pub fn consumer_resolution(&self) -> ConsumerResolution {
        self.consumer_resolution.clone()
    }

    pub fn consumer_publishing(&self) -> ConsumerPublishing {
        self.consumer_publishing.clone()
    }

    pub fn inlet_policy_expression(&self) -> Option<PolicyExpression> {
        self.inlet_policy_expression.clone()
    }

    pub fn consumer_policy_expression(&self) -> Option<PolicyExpression> {
        self.consumer_policy_expression.clone()
    }

    pub fn producer_policy_expression(&self) -> Option<PolicyExpression> {
        self.producer_policy_expression.clone()
    }
}

/// Request body when instructing a node to start an Uppercase service
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartUppercaseServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartUppercaseServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body when instructing a node to start an Echoer service
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartEchoerServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartEchoerServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body when instructing a node to start a Hop service
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartHopServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartHopServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body of remote proxy vault service
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartRemoteProxyVaultServiceRequest {
    #[n(1)] pub addr: String,
    #[n(2)] pub vault_name: String,
}

impl StartRemoteProxyVaultServiceRequest {
    pub fn new(addr: impl Into<String>, vault_name: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            vault_name: vault_name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceStatus {
    #[n(2)] pub addr: String,
    #[serde(rename = "type")]
    #[n(3)] pub service_type: String,
}

impl ServiceStatus {
    pub fn new(addr: impl Into<String>, service_type: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            service_type: service_type.into(),
        }
    }
}

impl Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} service at {}",
            color_warn(&self.service_type),
            color_primary(&self.addr)
        )
    }
}

impl Output for ServiceStatus {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}
