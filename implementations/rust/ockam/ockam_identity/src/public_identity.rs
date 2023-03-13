use crate::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use crate::{IdentityError, IdentityIdentifier, IdentityVault};
use ockam_core::compat::fmt;
use ockam_core::compat::fmt::{Display, Formatter};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::Signature;
use ockam_core::Result;
use ockam_vault::PublicKey;
use serde::{Deserialize, Serialize};

/// Public part of an `Identity`
#[derive(Clone, Serialize, Deserialize)]
pub struct PublicIdentity {
    id: IdentityIdentifier,
    change_history: IdentityChangeHistory,
}

impl PublicIdentity {
    pub(crate) fn new(id: IdentityIdentifier, change_history: IdentityChangeHistory) -> Self {
        Self { id, change_history }
    }

    /// Export to the binary format
    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    /// Import from the binary format
    pub async fn import(data: &[u8], vault: Arc<dyn IdentityVault>) -> Result<Self> {
        let change_history = IdentityChangeHistory::import(data)?;
        if !change_history
            .verify_all_existing_changes(vault.clone())
            .await?
        {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        let id = change_history.compute_identity_id(vault.clone()).await?;

        let identity = Self::new(id, change_history);

        Ok(identity)
    }

    pub(crate) fn changes(&self) -> &IdentityChangeHistory {
        &self.change_history
    }

    /// Compare to a previously known state of the same `Identity`
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        self.change_history.compare(&known.change_history)
    }

    /// `IdentityIdentifier`
    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub(crate) fn get_root_public_key(&self) -> Result<PublicKey> {
        self.change_history.get_root_public_key()
    }

    pub(crate) fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        self.change_history.get_public_key(label)
    }

    /// Verify signature using key with the given label
    pub async fn verify_signature(
        &self,
        signature: &Signature,
        data: &[u8],
        key_label: Option<&str>,
        vault: Arc<dyn IdentityVault>,
    ) -> Result<bool> {
        let public_key = match key_label {
            Some(label) => self.get_public_key(label)?,
            None => self.get_root_public_key()?,
        };

        vault.verify(signature, &public_key, data).await
    }
}

impl Display for PublicIdentity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let identifier = self.identifier();
        writeln!(f, "Identifier:     {identifier}")?;

        let history: Vec<u8> = self.export().map_err(|_| fmt::Error)?;
        let history_hex = hex::encode(history);
        writeln!(f, "Change history: {history_hex}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn test_display() {
        let data = hex::decode("0144c7eb72dd1e633f38e0d0521e9d5eb5072f6418176529eb1b00189e4d69ad2e000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020c6c52380125d42b0b4da922b1cff8503a258c3497ec8ac0b4a3baa0d9ca7b3780301014075064b902bda9d16db81ab5f38fbcf226a0e904e517a8c087d379ea139df1f2d7fee484ac7e1c2b7ab2da75f85adef6af7ddb05e7fa8faf180820cb9e86def02").unwrap();
        let public_identity = PublicIdentity::new(
            IdentityIdentifier::from_str(
                "Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50",
            )
            .unwrap(),
            IdentityChangeHistory::import(data.to_vec().as_slice()).unwrap(),
        );

        let actual = format!("{public_identity}");
        let expected = r#"Identifier:     Pfa804b7fca12a19eed206ae180b5b576860ae6512f196c189d90661bcc434b50
Change history: 0144c7eb72dd1e633f38e0d0521e9d5eb5072f6418176529eb1b00189e4d69ad2e000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020c6c52380125d42b0b4da922b1cff8503a258c3497ec8ac0b4a3baa0d9ca7b3780301014075064b902bda9d16db81ab5f38fbcf226a0e904e517a8c087d379ea139df1f2d7fee484ac7e1c2b7ab2da75f85adef6af7ddb05e7fa8faf180820cb9e86def02
"#;
        assert_eq!(actual, expected)
    }
}
