use crate::alloc::string::ToString;
use crate::authenticated_storage::{AttributesEntry, IdentityAttributeStorage};
use crate::credential::worker::CredentialExchangeWorker;
use crate::credential::{Credential, CredentialBuilder, CredentialData, Timestamp, Verified};
use crate::trust_context::AuthorityInfo;
use crate::{
    Identity, IdentityError, IdentityIdentifier, IdentitySecureChannelLocalInfo,
    IdentityStateConst, IdentityVault, PublicIdentity,
};
use core::marker::PhantomData;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{Address, AllowAll, AsyncTryClone, Error, Mailboxes, Result, Route};
use ockam_node::api::{request, request_with_local_info};
use ockam_node::WorkerBuilder;

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

    /// Start worker that will be available to receive others attributes and put them into storage,
    /// after successful verification
    pub async fn start_credential_exchange_worker(
        &self,
        address: impl Into<Address>,
        attributes_storage: Arc<dyn IdentityAttributeStorage>,
        authority_info: Arc<AuthorityInfo>,
    ) -> Result<()> {
        let s = self.async_try_clone().await?;
        let worker = CredentialExchangeWorker::new(s, attributes_storage, authority_info);

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
    pub async fn present_credential(
        &self,
        route: impl Into<Route>,
        credential: &Credential,
    ) -> Result<()> {
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
        authority: &PublicIdentity,
        attributes_storage: Arc<dyn IdentityAttributeStorage>,
        credential: &Credential,
    ) -> Result<()> {
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
            Some(s) => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    format!("credential presentation failed: {}", s),
                ))
            }
            _ => {
                return Err(Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    "credential presentation failed",
                ))
            }
        }

        let credential: Credential = dec.decode()?;

        self.receive_presented_credential(their_id, credential, authority, attributes_storage)
            .await?;

        Ok(())
    }
}

impl Identity {
    async fn verify_credential(
        sender: &IdentityIdentifier,
        credential: &Credential,
        authority: &PublicIdentity,
        vault: Arc<dyn IdentityVault>,
    ) -> Result<CredentialData<Verified>> {
        let credential_data = match authority
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
        authority: &PublicIdentity,
    ) -> Result<()> {
        let _ =
            Self::verify_credential(self.identifier(), credential, authority, self.vault.clone())
                .await?;
        Ok(())
    }

    pub(crate) async fn receive_presented_credential(
        &self,
        sender: IdentityIdentifier,
        credential: Credential,
        authority: &PublicIdentity,
        attributes_storage: Arc<dyn IdentityAttributeStorage>,
    ) -> Result<()> {
        let credential_data =
            Self::verify_credential(&sender, &credential, authority, self.vault.clone()).await?;

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
