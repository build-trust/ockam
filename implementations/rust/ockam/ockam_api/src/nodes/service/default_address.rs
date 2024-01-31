pub struct DefaultAddress;

impl DefaultAddress {
    pub const RELAY_SERVICE: &'static str = "forwarding_service";
    pub const UPPERCASE_SERVICE: &'static str = "uppercase";
    pub const ECHO_SERVICE: &'static str = "echo";
    pub const HOP_SERVICE: &'static str = "hop";
    pub const SECURE_CHANNEL_LISTENER: &'static str = "api";
    pub const DIRECT_AUTHENTICATOR: &'static str = "direct_authenticator";
    pub const CREDENTIAL_ISSUER: &'static str = "credential_issuer";
    pub const ENROLLMENT_TOKEN_ISSUER: &'static str = "enrollment_token_issuer";
    pub const ENROLLMENT_TOKEN_ACCEPTOR: &'static str = "enrollment_token_acceptor";
    pub const OKTA_IDENTITY_PROVIDER: &'static str = "okta";
    pub const KAFKA_OUTLET: &'static str = "kafka_outlet";
    pub const KAFKA_CONSUMER: &'static str = "kafka_consumer";
    pub const KAFKA_PRODUCER: &'static str = "kafka_producer";
    pub const KAFKA_DIRECT: &'static str = "kafka_direct";

    pub fn is_valid(name: &str) -> bool {
        matches!(name, |Self::RELAY_SERVICE| Self::UPPERCASE_SERVICE
            | Self::ECHO_SERVICE
            | Self::HOP_SERVICE
            | Self::SECURE_CHANNEL_LISTENER
            | Self::DIRECT_AUTHENTICATOR
            | Self::CREDENTIAL_ISSUER
            | Self::ENROLLMENT_TOKEN_ISSUER
            | Self::ENROLLMENT_TOKEN_ACCEPTOR
            | Self::OKTA_IDENTITY_PROVIDER
            | Self::KAFKA_CONSUMER
            | Self::KAFKA_PRODUCER
            | Self::KAFKA_OUTLET
            | Self::KAFKA_DIRECT)
    }

    pub fn iter() -> impl Iterator<Item = &'static str> {
        [
            Self::RELAY_SERVICE,
            Self::UPPERCASE_SERVICE,
            Self::ECHO_SERVICE,
            Self::HOP_SERVICE,
            Self::SECURE_CHANNEL_LISTENER,
            Self::DIRECT_AUTHENTICATOR,
            Self::CREDENTIAL_ISSUER,
            Self::ENROLLMENT_TOKEN_ISSUER,
            Self::ENROLLMENT_TOKEN_ACCEPTOR,
            Self::OKTA_IDENTITY_PROVIDER,
            Self::KAFKA_CONSUMER,
            Self::KAFKA_PRODUCER,
            Self::KAFKA_OUTLET,
            Self::KAFKA_DIRECT,
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
        assert!(DefaultAddress::is_valid(DefaultAddress::RELAY_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::UPPERCASE_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::ECHO_SERVICE));
        assert!(DefaultAddress::is_valid(DefaultAddress::HOP_SERVICE));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::SECURE_CHANNEL_LISTENER
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::DIRECT_AUTHENTICATOR
        ));
        assert!(DefaultAddress::is_valid(DefaultAddress::CREDENTIAL_ISSUER));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::ENROLLMENT_TOKEN_ISSUER
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR
        ));
        assert!(DefaultAddress::is_valid(
            DefaultAddress::OKTA_IDENTITY_PROVIDER
        ));
        assert!(DefaultAddress::is_valid(DefaultAddress::KAFKA_CONSUMER));
        assert!(DefaultAddress::is_valid(DefaultAddress::KAFKA_PRODUCER));
    }
}
