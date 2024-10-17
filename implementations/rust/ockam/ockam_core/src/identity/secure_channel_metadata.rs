use crate::compat::string::{String, ToString};
use crate::errcode::{Kind, Origin};
use crate::{AddressAndMetadata, Error, LocalInfoIdentifier, Result, SECURE_CHANNEL_IDENTIFIER};

/// SecureChannel Metadata used for Terminal Address
pub struct SecureChannelMetadata {
    their_identifier: LocalInfoIdentifier,
}

impl SecureChannelMetadata {
    /// Their Identifier
    pub fn their_identifier(&self) -> LocalInfoIdentifier {
        self.their_identifier.clone()
    }
}

impl SecureChannelMetadata {
    #[track_caller]
    fn error_type_id() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid metadata identifier for secure channel",
        )
    }

    #[track_caller]
    fn error_format() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid format for metadata identifier for secure channel",
        )
    }

    /// Get the Identifier of the other side of the Secure Channel
    pub fn from_terminal_address(terminal: &AddressAndMetadata) -> Result<Self> {
        let identifier = if let Some(identifier) =
            terminal
                .metadata
                .attributes
                .iter()
                .find_map(|(key, value)| {
                    if key == SECURE_CHANNEL_IDENTIFIER {
                        Some(value.clone())
                    } else {
                        None
                    }
                }) {
            identifier
        } else {
            return Err(Self::error_type_id());
        };

        if let Ok(identifier) = hex::decode(identifier) {
            match identifier.try_into() {
                Ok(identifier) => Ok(Self {
                    their_identifier: LocalInfoIdentifier(identifier),
                }),
                Err(_) => Err(Self::error_format()),
            }
        } else {
            Err(Self::error_format())
        }
    }

    /// Create an attribute for a Secure Channel given the Identifier of the other side
    pub fn attribute(their_identifier: LocalInfoIdentifier) -> (String, String) {
        (
            SECURE_CHANNEL_IDENTIFIER.to_string(),
            hex::encode(their_identifier.0),
        )
    }
}
