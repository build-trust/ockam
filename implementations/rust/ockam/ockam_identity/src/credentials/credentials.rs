use crate::credential::{Credential, CredentialData, Timestamp, Verified};
use crate::identities::{AttributesEntry, Identities};
use crate::identity::{Identity, IdentityError, IdentityIdentifier};
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{Error, Result};

/// This trait provides functions to issue, accept and verify credentials
#[async_trait]
pub trait Credentials: Send + Sync + 'static {
    /// Issue a credential for by having the issuer sign the serialized credential data
    async fn issue_credential(
        &self,
        issuer: &Identity,
        credential_data: CredentialData<Verified>,
    ) -> Result<Credential>;

    /// Verify that a credential has been signed by one of the authorities
    async fn verify_credential(
        &self,
        subject: &IdentityIdentifier,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<CredentialData<Verified>>;

    /// Verify and store a credential sent by a specific identity
    async fn receive_presented_credential(
        &self,
        sender: &IdentityIdentifier,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<()>;
}

#[async_trait]
impl Credentials for Identities {
    async fn verify_credential(
        &self,
        subject: &IdentityIdentifier,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<CredentialData<Verified>> {
        let credential_data = CredentialData::try_from(credential.data.as_slice())?;

        let issuer = authorities
            .iter()
            .find(|&x| x.identifier() == credential_data.issuer);
        let issuer = match issuer {
            Some(i) => i,
            None => return Err(IdentityError::UnknownAuthority.into()),
        };

        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Application, Kind::Invalid, "invalid system time"))?;

        credential_data.verify(subject, &issuer.identifier(), now)?;

        let sig = ockam_core::vault::Signature::new(credential.signature().to_vec());

        if !self
            .identities_keys()
            .verify_signature(
                issuer,
                &sig,
                credential.unverified_data(),
                Some(credential_data.clone().unverified_key_label()),
            )
            .await?
        {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "invalid signature",
            ));
        }
        Ok(credential_data.into_verified())
    }

    /// Create a signed credential based on the given values.
    async fn issue_credential(
        &self,
        issuer: &Identity,
        credential_data: CredentialData<Verified>,
    ) -> Result<Credential> {
        let bytes = minicbor::to_vec(credential_data)?;
        let sig = self
            .identities_keys()
            .create_signature(issuer, &bytes, None)
            .await?;
        Ok(Credential::new(bytes, SignatureVec::from(sig)))
    }

    async fn receive_presented_credential(
        &self,
        sender: &IdentityIdentifier,
        authorities: &[Identity],
        credential: Credential,
    ) -> Result<()> {
        let credential_data = self
            .verify_credential(sender, authorities, credential)
            .await?;

        self.identities_repository
            .put_attributes(
                sender,
                AttributesEntry::new(
                    credential_data.attributes.as_map_vec_u8(),
                    Timestamp::now().unwrap(),
                    Some(credential_data.expires),
                    Some(credential_data.issuer),
                ),
            )
            .await?;

        Ok(())
    }
}
