use super::super::models::{Change, ChangeHash, ChangeHistory, Identifier};
use super::super::IdentityError;
use super::verified_change::VerifiedChange;
use super::IdentityHistoryComparison;

use core::cmp::Ordering;
use core::fmt;
use core::fmt::{Display, Formatter};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{PublicKey, VerifyingVault};

/// Identity implementation
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
            Err(IdentityError::EmptyIdentity.into())
        }
    }
}

impl Identity {
    /// Export an `Identity` to the binary format
    pub fn export(&self) -> Result<Vec<u8>> {
        self.change_history.export()
    }

    /// Import and verify Identity from the ChangeHistory
    pub async fn import_from_change_history(
        expected_identifier: Option<&Identifier>,
        change_history: ChangeHistory,
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<Identity> {
        let verified_changes =
            Self::check_entire_consistency(&change_history.0, vault.clone()).await?;
        Self::verify_all_existing_changes(&verified_changes, &change_history.0, vault).await?;

        let identifier = if let Some(first_change) = verified_changes.first() {
            first_change.change_hash().clone().into()
        } else {
            return Err(IdentityError::IdentityVerificationFailed.into());
        };

        if let Some(expected_identifier) = expected_identifier {
            if &identifier != expected_identifier {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }
        }

        let identity = Self::new(identifier, verified_changes, change_history);

        Ok(identity)
    }

    /// Create an Identity from serialized data
    pub async fn import(
        expected_identifier: Option<&Identifier>,
        data: &[u8],
        vault: Arc<dyn VerifyingVault>,
    ) -> Result<Identity> {
        let change_history = ChangeHistory::import(data)?;

        Self::import_from_change_history(expected_identifier, change_history, vault).await
    }
}

impl Identity {
    /// Get latest public key
    pub fn get_public_key(&self) -> Result<PublicKey> {
        if let Some(last_change) = self.changes().last() {
            Ok(last_change.primary_public_key().clone())
        } else {
            Err(IdentityError::EmptyIdentity.into())
        }
    }
    /// Add a new key change to the change history
    pub async fn add_change(
        self,
        change: Change,
        vault: Arc<dyn VerifyingVault>,
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

        let history = hex::encode(self.export().map_err(|_| fmt::Error)?);
        writeln!(f, "Change history: {history}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::identities;
    use super::*;
    use core::str::FromStr;

    #[tokio::test]
    async fn test_display() {
        let data = hex::decode("81a201583ba20101025835a4028201815820bd144a3f6472ba2215b6b86b2820b23304f9473622847ca80dfda0d10f12eebc03f4041a64c956a9051a64c956a9028201815840c1598a6f85215c118a4744310bebfae71ec19353e1ede1582787592013d65a70c80aa4a4855d16d9b696a887be9bd97b2271245124857d67c07e0203564c3706").unwrap();
        let identity = identities()
            .identities_creation()
            .import(
                Some(&Identifier::from_str("Ie2424922b4194cd4ab57f952ef04c44e5e70ab2f").unwrap()),
                &data,
            )
            .await
            .unwrap();

        let actual = format!("{identity}");
        let expected = r#"Identifier:     Ie2424922b4194cd4ab57f952ef04c44e5e70ab2f
Change history: 81a201583ba20101025835a4028201815820bd144a3f6472ba2215b6b86b2820b23304f9473622847ca80dfda0d10f12eebc03f4041a64c956a9051a64c956a9028201815840c1598a6f85215c118a4744310bebfae71ec19353e1ede1582787592013d65a70c80aa4a4855d16d9b696a887be9bd97b2271245124857d67c07e0203564c3706
"#;
        assert_eq!(actual, expected)
    }

    #[tokio::test]
    async fn test_compare() -> Result<()> {
        let identities = identities();
        let identity1 = identities.identities_creation().create_identity().await?;

        let identity2 = identities
            .identities_keys()
            .rotate_key(identity1.clone())
            .await?;

        let identity3 = identities
            .identities_keys()
            .rotate_key(identity1.clone())
            .await?;

        assert_eq!(
            identity1.compare(&identity1),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity2.compare(&identity2),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity3.compare(&identity3),
            IdentityHistoryComparison::Equal
        );
        assert_eq!(
            identity1.compare(&identity2),
            IdentityHistoryComparison::Older
        );
        assert_eq!(
            identity2.compare(&identity1),
            IdentityHistoryComparison::Newer
        );
        assert_eq!(
            identity2.compare(&identity3),
            IdentityHistoryComparison::Conflict
        );

        Ok(())
    }
}
