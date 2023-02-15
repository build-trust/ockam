use crate::alloc::string::ToString;
use crate::authenticated_storage::{
    AttributesEntry, AuthenticatedStorage, IdentityAttributeStorage,
};
use crate::credential::worker::CredentialExchangeWorker;
use crate::credential::{
    Credential, CredentialBuilder, CredentialData, Timestamp, Unverified, Verified,
};
use crate::{
    Identity, IdentityError, IdentityIdentifier, IdentitySecureChannelLocalInfo,
    IdentityStateConst, IdentityVault, PublicIdentity,
};
use core::marker::PhantomData;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{Address, AllowAll, AsyncTryClone, Error, Mailboxes, Result, Route};
use ockam_node::api::{request, request_with_local_info};
use ockam_node::WorkerBuilder;

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    pub async fn set_credential(&self, credential: Credential) {
        // TODO: May also verify received credential calling self.verify_self_credential
        *self.credential.write().await = Some(credential);
    }

    pub async fn credential(&self) -> Option<Credential> {
        self.credential.read().await.clone()
    }

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

    /// Start worker that will be available to receive others attributes and put them into storage,
    /// after successful verification
    pub async fn start_credential_exchange_worker(
        &self,
        authorities: Vec<PublicIdentity>,
        address: impl Into<Address>,
        present_back: bool,
        attributes_storage: impl IdentityAttributeStorage,
    ) -> Result<()> {
        let s = self.async_try_clone().await?;
        let worker =
            CredentialExchangeWorker::new(authorities, present_back, s, attributes_storage);

        WorkerBuilder::with_mailboxes(
            Mailboxes::main(
                address.into(),
                Arc::new(AllowAll), // We check for Identity secure channel inside the worker
                Arc::new(AllowAll), // FIXME: @ac Allow to respond anywhere using return_route
            ),
            worker,
        )
        .start(&self.ctx)
        .await?;

        Ok(())
    }

    /// Present credential to other party, route shall use secure channel
    pub async fn present_credential(&self, route: impl Into<Route>) -> Result<()> {
        let credential = self.credential.read().await;
        let credential = credential.as_ref().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Invalid,
                "no credential to present",
            )
        })?;

        let buf = request(
            &self.ctx,
            "credential",
            None,
            route.into(),
            Request::post("actions/present").body(credential),
        )
        .await?;

        let res: Response = minicbor::decode(&buf)?;
        match res.status() {
            Some(Status::Ok) => Ok(()),
            _ => Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                "credential presentation failed",
            )),
        }
    }

    /// Present credential to other party, route shall use secure channel. Other party is expected
    /// to present its credential in response, otherwise this call errors.
    pub async fn present_credential_mutual(
        &self,
        route: impl Into<Route>,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        attributes_storage: &impl IdentityAttributeStorage,
    ) -> Result<()> {
        let credential = self.credential.read().await;
        let credential = credential.as_ref().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Invalid,
                "no credential to present",
            )
        })?;

        let path = "actions/present_mutual";
        let (buf, local_info) = request_with_local_info(
            &self.ctx,
            "credential",
            None,
            route.into(),
            Request::post(path).body(credential),
        )
        .await?;

        let their_id = IdentitySecureChannelLocalInfo::find_info_from_list(&local_info)?
            .their_identity_id()
            .clone();

        let mut dec = Decoder::new(&buf);
        let res: Response = dec.decode()?;
        match res.status() {
            Some(Status::Ok) => {}
            _ => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    "credential presentation failed",
                ))
            }
        }

        let credential: Credential = dec.decode()?;

        self.receive_presented_credential(their_id, credential, authorities, attributes_storage)
            .await?;

        Ok(())
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    async fn verify_credential(
        sender: &IdentityIdentifier,
        credential: &Credential,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        vault: &impl IdentityVault,
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

        let credential_data = match issuer.verify_credential(credential, sender, vault).await {
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
        let _ = Self::verify_credential(self.identifier(), credential, authorities, &self.vault)
            .await?;
        Ok(())
    }

    pub(crate) async fn receive_presented_credential(
        &self,
        sender: IdentityIdentifier,
        credential: Credential,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        attributes_storage: &impl IdentityAttributeStorage,
    ) -> Result<()> {
        let credential_data =
            Self::verify_credential(&sender, &credential, authorities, &self.vault).await?;

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
