use crate::{Identity, Profile, ProfileVault, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::compat::string::String;
use ockam_core::vault::PublicKey;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};

#[derive(AsyncTryClone)]
pub struct TrustPublicKeyPolicy<V: ProfileVault> {
    public_key: PublicKey,
    public_key_label: String,
    profile: Profile<V>,
}

impl<V: ProfileVault> TrustPublicKeyPolicy<V> {
    pub fn new(
        public_key: PublicKey,
        public_key_label: impl Into<String>,
        profile: Profile<V>,
    ) -> Self {
        Self {
            public_key,
            public_key_label: public_key_label.into(),
            profile,
        }
    }
}

#[async_trait]
impl<V: ProfileVault> TrustPolicy for TrustPublicKeyPolicy<V> {
    async fn check(&mut self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let contact;
        if let Some(c) = self
            .profile
            .get_contact(trust_info.their_profile_id())
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
