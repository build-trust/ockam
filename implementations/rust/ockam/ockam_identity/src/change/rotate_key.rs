use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::{IdentityChange, IdentitySignedChange, Signature, SignatureType};
use crate::change_history::IdentityChangeHistory;
use crate::{ChangeIdentifier, Identity, IdentityError, IdentityVault, KeyAttributes};
use core::fmt;
use ockam_core::vault::PublicKey;
use ockam_core::{Encodable, Result};
use serde::{Deserialize, Serialize};

/// RotateKeyChangeData
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RotateKeyChangeData {
    prev_change_id: ChangeIdentifier,
    key_attributes: KeyAttributes,
    public_key: PublicKey,
}

impl RotateKeyChangeData {
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

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(
        prev_change_id: ChangeIdentifier,
        key_attributes: KeyAttributes,
        public_key: PublicKey,
    ) -> Self {
        Self {
            prev_change_id,
            key_attributes,
            public_key,
        }
    }
}

impl fmt::Display for RotateKeyChangeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prev_change_id:{} key attibutes:{} public key:{}",
            self.prev_change_id(),
            self.key_attributes(),
            self.public_key()
        )
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// Rotate key change
    pub(crate) async fn make_rotate_key_change(
        &self,
        key_attributes: KeyAttributes,
    ) -> Result<IdentitySignedChange> {
        let change_history = self.change_history.read().await;
        let prev_change_id = change_history.get_last_change_id()?;

        let last_change_in_chain = IdentityChangeHistory::find_last_key_change(
            change_history.as_ref(),
            key_attributes.label(),
        )?
        .clone();

        let last_key_in_chain =
            Self::get_secret_key_from_change(&last_change_in_chain, &self.vault).await?;

        let secret_attributes = key_attributes.secret_attributes();

        let secret_key = self.vault.secret_generate(secret_attributes).await?;
        let public_key = self.vault.secret_public_key_get(&secret_key).await?;

        let data = RotateKeyChangeData::new(prev_change_id, key_attributes, public_key);

        let change_block = IdentityChange::RotateKey(data);
        let change_block_binary = change_block
            .encode()
            .map_err(|_| IdentityError::BareError)?;

        let change_id = self.vault.sha256(&change_block_binary).await?;
        let change_id = ChangeIdentifier::from_hash(change_id);

        let self_signature = self.vault.sign(&secret_key, change_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let root_key = self.get_root_secret_key().await?;

        let root_signature = self.vault.sign(&root_key, change_id.as_ref()).await?;
        let root_signature = Signature::new(SignatureType::RootSign, root_signature);

        let prev_signature = self
            .vault
            .sign(&last_key_in_chain, change_id.as_ref())
            .await?;
        let prev_signature = Signature::new(SignatureType::PrevSign, prev_signature);

        let signed_change = IdentitySignedChange::new(
            change_id,
            change_block,
            vec![self_signature, root_signature, prev_signature],
        );

        Ok(signed_change)
    }
}
