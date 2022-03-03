use crate::{Identity, IdentityTrait, IdentityVault, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::compat::string::String;
use ockam_core::vault::PublicKey;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};

#[derive(AsyncTryClone)]
pub struct TrustPublicKeyPolicy<V: IdentityVault> {
    public_key: PublicKey,
    public_key_label: String,
    identity: Identity<V>,
}

impl<V: IdentityVault> TrustPublicKeyPolicy<V> {
    pub fn new(
        public_key: PublicKey,
        public_key_label: impl Into<String>,
        identity: Identity<V>,
    ) -> Self {
        Self {
            public_key,
            public_key_label: public_key_label.into(),
            identity,
        }
    }
}

#[async_trait]
impl<V: IdentityVault> TrustPolicy for TrustPublicKeyPolicy<V> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let contact;
        if let Some(c) = self
            .identity
            .get_contact(trust_info.their_identity_id())
            .await?
        {
            contact = c;
        } else {
            return Ok(false);
        }

        if let Ok(pub_key) = contact.get_public_key(&self.public_key_label) {
            Ok(pub_key == self.public_key)
        } else {
            Ok(false)
        }
    }
}
