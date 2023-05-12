//! Identity history
use crate::identity::identity_change::IdentitySignedChange;
use crate::identity::identity_change_history::IdentityChangeHistory;
use crate::identity::identity_identifier::IdentityIdentifier;
use crate::IdentityHistoryComparison;
use core::fmt::{Display, Formatter};
use ockam_core::compat::fmt;
use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

/// Identity implementation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub(crate) identifier: IdentityIdentifier,
    pub(crate) change_history: IdentityChangeHistory,
}

impl Identity {
    /// Create a new identity
    pub fn new(identifier: IdentityIdentifier, change_history: IdentityChangeHistory) -> Self {
        Self {
            identifier,
            change_history,
        }
    }

    /// Return the identity identifier
    pub fn identifier(&self) -> IdentityIdentifier {
        self.identifier.clone()
    }

    /// Export an `Identity` to the binary format
    /// TODO: return a newtype instead of a raw vector
    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    /// Export an `Identity` as a hex-formatted string
    pub fn export_hex(&self) -> Result<String> {
        Ok(hex::encode(self.export()?))
    }

    /// Add a new key change to the change history
    pub fn add_change(&mut self, change: IdentitySignedChange) -> Result<()> {
        self.change_history.add_change(change)
    }

    /// `Identity` change history
    pub fn change_history(&self) -> IdentityChangeHistory {
        self.change_history.clone()
    }

    /// Return the root public key of an identity
    pub fn get_root_public_key(&self) -> Result<PublicKey> {
        self.change_history.get_root_public_key()
    }

    pub(crate) fn get_public_key(&self, key_label: Option<&str>) -> Result<PublicKey> {
        let key = match key_label {
            Some(label) => self.get_labelled_public_key(label)?,
            None => self.get_root_public_key()?,
        };
        Ok(key)
    }

    pub(crate) fn get_labelled_public_key(&self, label: &str) -> Result<PublicKey> {
        self.change_history.get_public_key(label)
    }

    /// Create an Identity from serialized data
    pub fn import(identifier: &IdentityIdentifier, data: &[u8]) -> Result<Identity> {
        let change_history = IdentityChangeHistory::import(data)?;
        Ok(Identity::new(identifier.clone(), change_history))
    }

    /// Return the list of key changes for this identity
    pub(crate) fn changes(&self) -> &IdentityChangeHistory {
        &self.change_history
    }

    /// Compare to a previously known state of the same `Identity`
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        self.change_history.compare(&known.change_history)
    }
}

impl Display for Identity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let identifier = self.identifier();
        writeln!(f, "Identifier:     {identifier}")?;

        let history: String = self.export_hex().map_err(|_| fmt::Error)?;
        writeln!(f, "Change history: {history}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let data = hex::decode("0144c7eb72dd1e633f38e0d0521e9d5eb5072f6418176529eb1b00189e4d69ad2e000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020c6c52380125d42b0b4da922b1cff8503a258c3497ec8ac0b4a3baa0d9ca7b3780301014075064b902bda9d16db81ab5f38fbcf226a0e904e517a8c087d379ea139df1f2d7fee484ac7e1c2b7ab2da75f85adef6af7ddb05e7fa8faf180820cb9e86def02").unwrap();
        let identity = Identity::new(
            IdentityIdentifier::from_hex(
                "fa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50",
            ),
            IdentityChangeHistory::import(data.to_vec().as_slice()).unwrap(),
        );

        let actual = format!("{identity}");
        let expected = r#"Identifier:     Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50
Change history: 0144c7eb72dd1e633f38e0d0521e9d5eb5072f6418176529eb1b00189e4d69ad2e000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020c6c52380125d42b0b4da922b1cff8503a258c3497ec8ac0b4a3baa0d9ca7b3780301014075064b902bda9d16db81ab5f38fbcf226a0e904e517a8c087d379ea139df1f2d7fee484ac7e1c2b7ab2da75f85adef6af7ddb05e7fa8faf180820cb9e86def02
"#;
        assert_eq!(actual, expected)
    }
}
