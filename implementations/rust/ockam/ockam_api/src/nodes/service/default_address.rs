use ockam::udp::UDP;
use ockam_core::Address;

pub struct DefaultAddress;

impl DefaultAddress {
    pub const OUTLET_SERVICE: &'static str = "outlet";
    pub const RELAY_SERVICE: &'static str = "forwarding_service";
    pub const STATIC_RELAY_SERVICE: &'static str = "static_forwarding_service";
    pub const UPPERCASE_SERVICE: &'static str = "uppercase";
    pub const ECHO_SERVICE: &'static str = "echo";
    pub const HOP_SERVICE: &'static str = "hop";
    pub const REMOTE_PROXY_VAULT: &'static str = "remote_proxy_vault";
    pub const SECURE_CHANNEL_LISTENER: &'static str = "api";
    pub const KEY_EXCHANGER_LISTENER: &'static str = "key_exchanger";
    pub const UDP_PUNCTURE_NEGOTIATION_LISTENER: &'static str = "udp";
    pub const RENDEZVOUS_SERVICE: &'static str = "rendezvous";
    pub const DIRECT_AUTHENTICATOR: &'static str = "direct_authenticator";
    pub const CREDENTIAL_ISSUER: &'static str = "credential_issuer";
    pub const ENROLLMENT_TOKEN_ISSUER: &'static str = "enrollment_token_issuer";
    pub const ENROLLMENT_TOKEN_ACCEPTOR: &'static str = "enrollment_token_acceptor";
    pub const OKTA_IDENTITY_PROVIDER: &'static str = "okta";
    pub const KAFKA_OUTLET: &'static str = "kafka_outlet";
    pub const KAFKA_INLET: &'static str = "kafka_inlet";
    pub const LEASE_MANAGER: &'static str = "lease_manager";

    pub fn get_rendezvous_server_address() -> Address {
        let server_address = std::env::var("OCKAM_RENDEZVOUS_SERVER")
            .unwrap_or("rendezvous.orchestrator.ockam.io:443".to_string());
        (UDP, server_address).into()
    }

    pub fn is_valid(name: &str) -> bool {
        matches!(name, |Self::OUTLET_SERVICE| Self::RELAY_SERVICE
            | Self::STATIC_RELAY_SERVICE
            | Self::UPPERCASE_SERVICE
            | Self::ECHO_SERVICE
            | Self::HOP_SERVICE
            | Self::REMOTE_PROXY_VAULT
            | Self::SECURE_CHANNEL_LISTENER
            | Self::KEY_EXCHANGER_LISTENER
            | Self::DIRECT_AUTHENTICATOR
            | Self::CREDENTIAL_ISSUER
            | Self::ENROLLMENT_TOKEN_ISSUER
            | Self::ENROLLMENT_TOKEN_ACCEPTOR
            | Self::OKTA_IDENTITY_PROVIDER
            | Self::KAFKA_INLET
            | Self::KAFKA_OUTLET
            | Self::LEASE_MANAGER)
    }

    pub fn iter() -> impl Iterator<Item = &'static str> {
        [
            Self::OUTLET_SERVICE,
            Self::RELAY_SERVICE,
            Self::STATIC_RELAY_SERVICE,
            Self::UPPERCASE_SERVICE,
            Self::ECHO_SERVICE,
            Self::HOP_SERVICE,
            Self::SECURE_CHANNEL_LISTENER,
            Self::KEY_EXCHANGER_LISTENER,
            Self::DIRECT_AUTHENTICATOR,
            Self::CREDENTIAL_ISSUER,
            Self::ENROLLMENT_TOKEN_ISSUER,
            Self::ENROLLMENT_TOKEN_ACCEPTOR,
            Self::OKTA_IDENTITY_PROVIDER,
            Self::KAFKA_INLET,
            Self::KAFKA_OUTLET,
            Self::LEASE_MANAGER,
        ]
        .iter()
        .copied()
    }
}

#[cfg(test)]
mod test {
    use super::DefaultAddress;

    #[test]
    fn test_default_address_is_valid() {
        assert!(!DefaultAddress::is_valid("foo"));
        for name in DefaultAddress::iter() {
            assert!(DefaultAddress::is_valid(name));
        }
    }
}
