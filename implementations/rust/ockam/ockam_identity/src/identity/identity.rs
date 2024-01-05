use core::cmp::Ordering;
use core::fmt;
use core::fmt::{Display, Formatter};

use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{VaultForVerifyingSignatures, VerifyingPublicKey};

use crate::models::{Change, ChangeHash, ChangeHistory, Identifier};
use crate::verified_change::VerifiedChange;
use crate::IdentityHistoryComparison;
use crate::{IdentityError, Vault};

/// Verified Identity
#[derive(Clone, Debug)]
pub struct Identity {
    identifier: Identifier,
    changes: Vec<VerifiedChange>,
    // We preserve the original change_history binary
    // as serialization is not guaranteed to be deterministic
    change_history: ChangeHistory,
}

impl Eq for Identity {}

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        self.change_history == other.change_history
    }
}

impl Identity {
    /// Create a new identity
    /// NOTE: This is intentionally private, so that the only way to create such struct is by
    /// going through the verification process
    fn new(
        identifier: Identifier,
        changes: Vec<VerifiedChange>,
        change_history: ChangeHistory,
    ) -> Self {
        Self {
            identifier,
            changes,
            change_history,
        }
    }

    /// Return the identity identifier
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    /// Collection of parsed changes
    pub fn changes(&self) -> &[VerifiedChange] {
        self.changes.as_slice()
    }

    /// `Identity` change history
    pub fn change_history(&self) -> &ChangeHistory {
        &self.change_history
    }

    /// `Identity`'s latest [`ChangeHash`]
    pub fn latest_change_hash(&self) -> Result<&ChangeHash> {
        if let Some(latest_change) = self.changes.last() {
            Ok(latest_change.change_hash())
        } else {
            Err(IdentityError::EmptyIdentity)?
        }
    }
}

impl Identity {
    /// Export an `Identity` to the binary format
    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    /// Export an `Identity` to a hex-encoded string
    pub fn export_as_string(&self) -> Result<String> {
        self.change_history.export_as_string()
    }

    /// Import and verify Identity from the ChangeHistory as a string
    pub async fn import_from_string(
        expected_identifier: Option<&Identifier>,
        change_history: &str,
        vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Result<Identity> {
        let change_history = ChangeHistory::import_from_string(change_history)?;
        Self::import_from_change_history(expected_identifier, change_history, vault).await
    }

    /// Create an identity for its change history as a hex-encoded string
    pub async fn create(change_history: &str) -> Result<Identity> {
        Self::import_from_string(None, change_history, Vault::create_verifying_vault()).await
    }

    /// Create an identity for its change history
    pub async fn create_from_change_history(change_history: &ChangeHistory) -> Result<Identity> {
        Self::import_from_change_history(
            None,
            change_history.clone(),
            Vault::create_verifying_vault(),
        )
        .await
    }

    /// Import and verify Identity from the ChangeHistory
    pub async fn import_from_change_history(
        expected_identifier: Option<&Identifier>,
        change_history: ChangeHistory,
        vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Result<Identity> {
        let verified_changes =
            Self::check_entire_consistency(&change_history.0, vault.clone()).await?;
        Self::verify_all_existing_changes(&verified_changes, &change_history.0, vault).await?;

        let identifier = if let Some(first_change) = verified_changes.first() {
            first_change.change_hash().clone().into()
        } else {
            return Err(IdentityError::IdentityVerificationFailed)?;
        };

        if let Some(expected_identifier) = expected_identifier {
            if &identifier != expected_identifier {
                return Err(IdentityError::IdentityVerificationFailed)?;
            }
        }

        let identity = Self::new(identifier, verified_changes, change_history);

        Ok(identity)
    }

    /// Create an Identity from serialized data
    pub async fn import(
        expected_identifier: Option<&Identifier>,
        data: &[u8],
        vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Result<Identity> {
        let change_history = ChangeHistory::import(data)?;

        Self::import_from_change_history(expected_identifier, change_history, vault).await
    }
}

impl Identity {
    /// Get latest public key
    pub fn get_latest_public_key(&self) -> Result<VerifyingPublicKey> {
        if let Some(last_change) = self.changes().last() {
            Ok(last_change.primary_public_key().clone())
        } else {
            Err(IdentityError::EmptyIdentity)?
        }
    }

    /// Get latest [`VerifiedChange`]
    pub fn get_latest_change(&self) -> Result<VerifiedChange> {
        if let Some(last_change) = self.changes().last() {
            Ok(last_change.clone())
        } else {
            Err(IdentityError::EmptyIdentity)?
        }
    }

    /// Add a new key change to the change history
    pub async fn add_change(
        self,
        change: Change,
        vault: Arc<dyn VaultForVerifyingSignatures>,
    ) -> Result<Identity> {
        // TODO: Optimize
        let mut change_history = self.change_history;
        change_history.0.push(change);

        Self::import_from_change_history(None, change_history, vault).await
    }

    /// Compare to a previously known state of the same `Identity`
    pub fn compare(&self, known: &Self) -> IdentityHistoryComparison {
        for change_pair in self.changes.iter().zip(known.changes.iter()) {
            if change_pair.0.change_hash() != change_pair.1.change_hash() {
                return IdentityHistoryComparison::Conflict;
            }
        }

        match self.changes.len().cmp(&known.changes.len()) {
            Ordering::Less => IdentityHistoryComparison::Older,
            Ordering::Equal => IdentityHistoryComparison::Equal,
            Ordering::Greater => IdentityHistoryComparison::Newer,
        }
    }
}

impl Display for Identity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let identifier = self.identifier();
        writeln!(f, "Identifier:     {identifier}")?;

        self.change_history.fmt(f)
    }
}

impl Display for ChangeHistory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let history = hex::encode(self.export().map_err(|_| fmt::Error)?);
        writeln!(f, "Change history: {history}")
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use rand::thread_rng;

    use ockam_core::compat::rand::RngCore;
    use ockam_vault::{EdDSACurve25519SecretKey, SigningSecret, SoftwareVaultForSigning};

    use crate::{identities, Identities, Vault};

    use super::*;

    #[tokio::test]
    async fn test_display() -> Result<()> {
        let identities = identities().await?;
        let data = hex::decode("81825837830101583285f68200815820f405e06d988fa8039cce1cd0ae607e46847c1b64bc459ca9d89dd9b21ae30681f41a654cebe91a7818eee98200815840494c9b70e8a9ad5593fceb478f722a513b4bd39fa70f4265d584253bc24617d0eb498ce532273f6d0d5326921e013696fce57c20cc6c4008f74b816810f0b009").unwrap();
        let identifier = identities
            .identities_creation()
            .import(
                Some(
                    &Identifier::from_str(
                        "I923829d0397a06fa862be5a87b7966959b8ef99ab6455b843ca9131a747b4819",
                    )
                    .unwrap(),
                ),
                &data,
            )
            .await
            .unwrap();
        let identity = identities.get_identity(&identifier).await.unwrap();

        let actual = format!("{identity}");
        let expected = r#"Identifier:     I923829d0397a06fa862be5a87b7966959b8ef99ab6455b843ca9131a747b4819
Change history: 81825837830101583285f68200815820f405e06d988fa8039cce1cd0ae607e46847c1b64bc459ca9d89dd9b21ae30681f41a654cebe91a7818eee98200815840494c9b70e8a9ad5593fceb478f722a513b4bd39fa70f4265d584253bc24617d0eb498ce532273f6d0d5326921e013696fce57c20cc6c4008f74b816810f0b009
"#;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_compare() -> Result<()> {
        let signing_vault0 = SoftwareVaultForSigning::create().await?;
        let signing_vault01 = SoftwareVaultForSigning::create().await?;
        let signing_vault02 = SoftwareVaultForSigning::create().await?;

        let mut key0_bin = [0u8; 32];
        thread_rng().fill_bytes(&mut key0_bin);

        let key0 = signing_vault0
            .import_key(SigningSecret::EdDSACurve25519(
                EdDSACurve25519SecretKey::new(key0_bin),
            ))
            .await?;
        let key01 = signing_vault01
            .import_key(SigningSecret::EdDSACurve25519(
                EdDSACurve25519SecretKey::new(key0_bin),
            ))
            .await?;
        let key02 = signing_vault02
            .import_key(SigningSecret::EdDSACurve25519(
                EdDSACurve25519SecretKey::new(key0_bin),
            ))
            .await?;

        let identities0 = Identities::builder()
            .await?
            .with_vault(Vault::new(
                signing_vault0,
                Vault::create_secure_channel_vault().await?,
                Vault::create_credential_vault().await?,
                Vault::create_verifying_vault(),
            ))
            .build();

        let identifier0 = identities0
            .identities_creation()
            .identity_builder()
            .with_existing_key(key0)
            .build()
            .await?;
        let identity0 = identities0.get_identity(&identifier0).await?;
        let identity0_bin = identities0.export_identity(&identifier0).await?;

        let identities01 = Identities::builder()
            .await?
            .with_vault(Vault::new(
                signing_vault01,
                Vault::create_secure_channel_vault().await?,
                Vault::create_credential_vault().await?,
                Vault::create_verifying_vault(),
            ))
            .build();
        let identities02 = Identities::builder()
            .await?
            .with_vault(Vault::new(
                signing_vault02,
                Vault::create_secure_channel_vault().await?,
                Vault::create_credential_vault().await?,
                Vault::create_verifying_vault(),
            ))
            .build();

        let identifier01 = identities01
            .identities_creation()
            .import_private_identity(Some(&identifier0), &identity0_bin, &key01)
            .await?;
        assert_eq!(identifier01, identifier0);
        let identifier02 = identities02
            .identities_creation()
            .import_private_identity(Some(&identifier0), &identity0_bin, &key02)
            .await?;
        assert_eq!(identifier02, identifier0);

        identities01
            .identities_creation()
            .rotate_identity(&identifier0)
            .await?;
        let identity01 = identities01.get_identity(&identifier0).await?;

        identities02
            .identities_creation()
            .rotate_identity(&identifier0)
            .await?;
        let identity02 = identities02.get_identity(&identifier0).await?;

        assert_eq!(
            identity0.compare(&identity0),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity01.compare(&identity01),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity02.compare(&identity02),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity0.compare(&identity01),
            IdentityHistoryComparison::Older
        );
        assert_eq!(
            identity0.compare(&identity02),
            IdentityHistoryComparison::Older
        );
        assert_eq!(
            identity01.compare(&identity0),
            IdentityHistoryComparison::Newer
        );
        assert_eq!(
            identity02.compare(&identity0),
            IdentityHistoryComparison::Newer
        );
        assert_eq!(
            identity01.compare(&identity02),
            IdentityHistoryComparison::Conflict
        );

        Ok(())
    }
}
