use crate::authenticated_storage::AuthenticatedStorage;
use crate::credential::worker::CredentialExchangeWorker;
use crate::credential::{
    AttributesEntry, AttributesStorageUtils, Credential, CredentialBuilder, CredentialData,
    Timestamp, Unverified, Verified,
};
use crate::{
    Identity, IdentityIdentifier, IdentitySecureChannelLocalInfo, IdentityStateConst,
    IdentityVault, PublicIdentity,
};
use core::marker::PhantomData;
use minicbor::Decoder;
use ockam_core::api::{Request, Response, Status};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{Address, AsyncTryClone, CowStr, Error, Result, Route};
use ockam_node::api::{request, request_with_local_info};

impl<V: IdentityVault> Identity<V> {
    pub async fn set_credential(&self, credential: Option<Credential<'static>>) {
        // TODO: May also verify received credential calling self.verify_self_credential
        *self.credential.write().await = credential;
    }

    pub async fn credential(&self) -> Option<Credential<'_>> {
        self.credential.read().await.clone()
    }

    /// Create a signed credential based on the given values.
    pub async fn issue_credential<'a>(
        &self,
        builder: CredentialBuilder<'a>,
    ) -> Result<Credential<'a>> {
        let key_label = IdentityStateConst::ROOT_LABEL;
        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Core, Kind::Internal, "invalid system time"))?;
        let exp = Timestamp(u64::from(now).saturating_add(builder.validity.as_secs()));
        let dat = CredentialData {
            schema: builder.schema,
            attributes: builder.attrs,
            subject: builder.subject,
            issuer: self.identifier().clone(),
            issuer_key_label: CowStr(key_label.into()),
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
    pub async fn start_credentials_exchange_worker(
        &self,
        authorities: Vec<PublicIdentity>,
        address: impl Into<Address>,
        present_back: bool,
        authenticated_storage: impl AuthenticatedStorage,
    ) -> Result<()> {
        let s = self.async_try_clone().await?;
        let worker =
            CredentialExchangeWorker::new(authorities, present_back, authenticated_storage, s);

        self.ctx.start_worker(address.into(), worker).await
    }

    /// Present credential to other party, route shall use secure channel
    pub async fn present_credential(&self, route: impl Into<Route>) -> Result<()> {
        let credentials = self.credential.read().await;
        let credential = credentials.as_ref().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Invalid,
                "no credential to present",
            )
        })?;

        let mut child_ctx = self.ctx.new_detached(Address::random_local()).await?;
        let buf = request(
            &mut child_ctx,
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
        authenticated_storage: &impl AuthenticatedStorage,
    ) -> Result<()> {
        let credentials = self.credential.read().await;
        let credential = credentials.as_ref().ok_or_else(|| {
            Error::new(
                Origin::Application,
                Kind::Invalid,
                "no credential to present",
            )
        })?;

        let mut child_ctx = self.ctx.new_detached(Address::random_local()).await?;
        let path = "actions/present_mutual";
        let (buf, local_info) = request_with_local_info(
            &mut child_ctx,
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

        let res = self
            .receive_presented_credential(their_id, credential, authorities, authenticated_storage)
            .await?;

        match res {
            ProcessArrivedCredentialResult::Ok() => Ok(()),
            ProcessArrivedCredentialResult::BadRequest(str) => {
                Err(Error::new(Origin::Application, Kind::Protocol, str))
            }
        }
    }
}

pub(crate) enum ProcessArrivedCredentialResult {
    Ok(),
    BadRequest(&'static str),
}

impl<V: IdentityVault> Identity<V> {
    pub(crate) async fn receive_presented_credential(
        &self,
        sender: IdentityIdentifier,
        credential: Credential<'_>,
        authorities: impl IntoIterator<Item = &PublicIdentity>,
        authenticated_storage: &impl AuthenticatedStorage,
    ) -> Result<ProcessArrivedCredentialResult> {
        let credential_data: CredentialData<Unverified> = match minicbor::decode(&credential.data) {
            Ok(c) => c,
            Err(_) => {
                return Ok(ProcessArrivedCredentialResult::BadRequest(
                    "invalid credential",
                ))
            }
        };

        let issuer = authorities
            .into_iter()
            .find(|&x| x.identifier() == &credential_data.issuer);
        let issuer = match issuer {
            Some(i) => i,
            None => {
                return Ok(ProcessArrivedCredentialResult::BadRequest(
                    "unknown authority",
                ));
            }
        };

        let credential_data = match issuer
            .verify_credential(&credential, &sender, &self.vault)
            .await
        {
            Ok(d) => d,
            Err(_) => {
                return Ok(ProcessArrivedCredentialResult::BadRequest(
                    "credential verification failed",
                ))
            }
        };

        AttributesStorageUtils::put_attributes(
            &sender,
            AttributesEntry::new(credential_data.attributes, credential_data.expires),
            authenticated_storage,
        )
        .await?;

        Ok(ProcessArrivedCredentialResult::Ok())
    }
}
