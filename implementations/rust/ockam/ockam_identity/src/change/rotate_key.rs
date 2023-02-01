use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::{IdentityChange, IdentityChangeSignature, IdentitySignedChange, SignatureType};
use crate::change_history::IdentityChangeHistory;
use crate::{ChangeIdentifier, Identity, IdentityError, IdentityVault};
use core::fmt;
use ockam_core::compat::string::String;
use ockam_core::vault::{KeyAttributes, PublicKey};
use ockam_core::{Encodable, Result};
use serde::{Deserialize, Serialize};

impl RotateKeyChangeData {
    /// Return key label
    pub fn key_label(&self) -> &String {
        &self.key_label
    }
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    /// Previous change identifier, used to create a chain
    pub fn prev_change_id(&self) -> &ChangeIdentifier {
        &self.prev_change_id
    }
}

/// RotateKeyChangeData
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    prev_change_id: ChangeIdentifier,
    key_label: String,
    key_attributes: KeyAttributes,
    public_key: PublicKey,
}

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(
        prev_change_id: ChangeIdentifier,
        key_label: String,
        key_attributes: KeyAttributes,
        public_key: PublicKey,
    ) -> Self {
        Self {
            prev_change_id,
            key_label,
            key_attributes,
            public_key,
        }
    }
}

impl fmt::Display for RotateKeyChangeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prev_change_id:{} key label:{} key attributes:{} public key:{}",
            self.prev_change_id(),
            self.key_label(),
            self.key_attributes(),
            self.public_key()
        )
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// Rotate key change
    pub(crate) async fn make_rotate_key_change(
        &self,
        key_label: String,
        key_attributes: KeyAttributes,
    ) -> Result<IdentitySignedChange> {
        let change_history = self.change_history.read().await;
        let prev_change_id = change_history.get_last_change_id()?;

        let last_change_in_chain = IdentityChangeHistory::find_last_key_change(
            change_history.as_ref(),
            key_label.as_str(),
        )?
        .clone();

        let last_key_in_chain =
            Self::get_secret_key_from_change(&last_change_in_chain, &self.vault).await?;

        let secret_key = self.vault.generate_key(key_attributes).await?;
        let public_key = self.vault.get_public_key(&secret_key).await?;

        let data = RotateKeyChangeData::new(
            prev_change_id,
            key_label.clone(),
            key_attributes,
            public_key,
        );

        let change_block = IdentityChange::RotateKey(data);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = self.vault.sha256(&change_block_binary).await?;
        let change_id = ChangeIdentifier::from_hash(change_id);

        let self_signature = self.vault.sign(&secret_key, change_id.as_ref()).await?;
        let self_signature = IdentityChangeSignature::new(SignatureType::SelfSign, self_signature);

        let root_key = self.get_root_secret_key().await?;

        let root_signature = self.vault.sign(&root_key, change_id.as_ref()).await?;
        let root_signature = IdentityChangeSignature::new(SignatureType::RootSign, root_signature);

        let prev_signature = self
            .vault
            .sign(&last_key_in_chain, change_id.as_ref())
            .await?;
        let prev_signature = IdentityChangeSignature::new(SignatureType::PrevSign, prev_signature);

        let signed_change = IdentitySignedChange::new(
            change_id,
            change_block,
            vec![self_signature, root_signature, prev_signature],
        );

        Ok(signed_change)
    }
}
