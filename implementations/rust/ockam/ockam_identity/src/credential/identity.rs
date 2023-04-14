use crate::alloc::string::ToString;
use crate::authenticated_storage::{AttributesEntry, IdentityAttributeStorage};
use crate::credential::{
    Credential, CredentialBuilder, CredentialData, Timestamp, Unverified, Verified,
};
use crate::{
    Identity, IdentityError, IdentityIdentifier, IdentityStateConst, IdentityVault, PublicIdentity,
};
use core::marker::PhantomData;
use ockam_core::compat::sync::Arc;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{Address, AllowAll, AsyncTryClone, Error, Mailboxes, Result, Route};
use ockam_node::api::{request, request_with_local_info};
use ockam_node::{MessageSendReceiveOptions, WorkerBuilder};

impl Identity {
    /// Create a signed credential based on the given values.
    pub async fn issue_credential(&self, builder: CredentialBuilder) -> Result<Credential> {
        let key_label = IdentityStateConst::ROOT_LABEL;
        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Core, Kind::Internal, "invalid system time"))?;
        let exp = Timestamp(u64::from(now).saturating_add(builder.validity.as_secs()));
        let dat = CredentialData {
            schema: builder.schema,
            attributes: builder.attrs,
            subject: builder.subject,
            issuer: self.identifier().clone(),
            issuer_key_label: key_label.into(),
            created: now,
            expires: exp,
            status: None::<PhantomData<Verified>>,
        };
        let bytes = minicbor::to_vec(&dat)?;

        let sig = self.create_signature(&bytes, None).await?;
        Ok(Credential::new(bytes, SignatureVec::from(sig)))
    }
}

impl Identity {
    async fn verify_credential(
        sender: &IdentityIdentifier,
        credential: &Credential,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        vault: Arc<dyn IdentityVault>,
    ) -> Result<CredentialData<Verified>> {
        let credential_data: CredentialData<Unverified> = match minicbor::decode(&credential.data) {
            Ok(c) => c,
            Err(_) => return Err(IdentityError::InvalidCredentialFormat.into()),
        };

        let issuer = authorities
            .into_iter()
            .find(|&x| x.identifier() == &credential_data.issuer);
        let issuer = match issuer {
            Some(i) => i,
            None => return Err(IdentityError::UnknownAuthority.into()),
        };

        let credential_data = match issuer
            .verify_credential(credential, sender, vault.clone())
            .await
        {
            Ok(d) => d,
            Err(_) => return Err(IdentityError::CredentialVerificationFailed.into()),
        };

        Ok(credential_data)
    }

    pub async fn verify_self_credential(
        &self,
        credential: &Credential,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
    ) -> Result<()> {
        let _ = Self::verify_credential(
            self.identifier(),
            credential,
            authorities,
            self.vault.clone(),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn receive_presented_credential(
        &self,
        sender: IdentityIdentifier,
        credential: Credential,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        attributes_storage: Arc<dyn IdentityAttributeStorage>,
    ) -> Result<()> {
        let credential_data =
            Self::verify_credential(&sender, &credential, authorities, self.vault.clone()).await?;

        //TODO: review the credential' attributes types.   They are references and has lifetimes,
        //etc,  but in reality this is always just deserizalided (either from wire or from
        //storage), so imho all that just add to the complexity without gaining much
        let attrs = credential_data
            .attributes
            .attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_vec()))
            .collect();
        attributes_storage
            .put_attributes(
                &sender,
                AttributesEntry::new(
                    attrs,
                    Timestamp::now().unwrap(),
                    Some(credential_data.expires),
                    Some(credential_data.issuer),
                ),
            )
            .await?;

        Ok(())
    }
}
