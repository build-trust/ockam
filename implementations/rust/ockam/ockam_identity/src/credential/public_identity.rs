use crate::alloc::borrow::ToOwned;
use crate::authenticated_storage::IdentityAttributeStorage;
use crate::credential::{Credential, CredentialData, Timestamp, Verified};
use crate::PublicIdentity;
use crate::{IdentityIdentifier, IdentityStateConst, IdentityVault};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::Signature;
use ockam_core::{Error, Result};

impl PublicIdentity {
    /// Perform a signature check with the given identity.
    ///
    /// If successful, the credential data are returned.
    pub async fn verify_credential<'a, 'b: 'a>(
        &self,
        credential: &'b Credential<'b>,
        subject: &IdentityIdentifier,
        vault: &impl IdentityVault,
    ) -> Result<CredentialData<'a, Verified>> {
        let dat = CredentialData::try_from(credential)?;
        if dat.unverfied_key_label() != IdentityStateConst::ROOT_LABEL {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signing key",
            ));
        }

        if &dat.issuer != self.identifier() {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "unknown authority",
            ));
        }

        if &dat.subject != subject {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "unknown subject",
            ));
        }

        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Application, Kind::Invalid, "invalid system time"))?;
        if dat.expires <= now {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "expired credential",
            ));
        }

        let sig = Signature::new(credential.signature().to_vec());

        if !self
            .verify_signature(
                &sig,
                credential.unverified_data(),
                Some(dat.unverfied_key_label()),
                vault,
            )
            .await?
        {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signature",
            ));
        }
        Ok(dat.into_verified())
    }

    /// Return authenticated non-expired attributes attached to that Identity
    pub async fn get_attributes(
        &self,
        authenticated_storage: &impl IdentityAttributeStorage,
    ) -> Result<Option<BTreeMap<String, Vec<u8>>>> {
        authenticated_storage
            .get_attributes(self.identifier())
            .await
            .map(|r| r.map(|e| e.attrs().to_owned()))
    }
}
