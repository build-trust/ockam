use crate::authenticated_storage::AuthenticatedStorage;
use crate::credential::{Attributes, Credential, CredentialData, Timestamp, Unverified};
use crate::error::IdentityError;
use crate::{
    IdentityIdentifier, IdentitySecureChannelLocalInfo, IdentityStateConst, IdentityVault,
    PublicIdentity,
};
use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Decoder, Encode};
use ockam_core::api::Method::Post;
use ockam_core::api::{Error, Id, Request, Response, ResponseBuilder, Status};
use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::Verifier;
use ockam_core::{Address, Result, Route, Routed, Worker};
use ockam_node::api::{request, request_with_local_info};
use ockam_node::Context;
use tracing::{error, trace, warn};

const TARGET: &str = "ockam::credential_exchange_worker::service";

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry<'a> {
    #[b(1)] attrs: Attributes<'a>,
    #[n(2)] expires: Timestamp,
}

impl<'a> AttributesEntry<'a> {
    pub fn new(attrs: Attributes<'a>, expires: Timestamp) -> Self {
        Self { attrs, expires }
    }
    pub fn attrs(&self) -> &Attributes<'a> {
        &self.attrs
    }
    pub fn expires(&self) -> Timestamp {
        self.expires
    }
}

enum ProcessArrivedCredentialResult {
    Ok(),
    BadRequest(&'static str),
}

pub struct CredentialExchange {
    ctx: Context,
}

impl CredentialExchange {
    pub async fn create(ctx: &Context) -> Result<Self> {
        Ok(Self {
            ctx: ctx.new_detached(Address::random_local()).await?,
        })
    }

    /// Create a generic bad request response.
    pub fn bad_request<'a>(id: Id, path: &'a str, msg: &'a str) -> ResponseBuilder<Error<'a>> {
        let e = Error::new(path).with_message(msg);
        Response::bad_request(id).body(e)
    }

    async fn receive_presented_credential(
        sender: IdentityIdentifier,
        credential: Credential<'_>,
        authorities: &BTreeMap<IdentityIdentifier, PublicIdentity>,
        vault: &impl IdentityVault,
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

        if credential_data.subject != sender {
            return Ok(ProcessArrivedCredentialResult::BadRequest(
                "unknown authority",
            ));
        }

        let now = Timestamp::now().ok_or_else(|| {
            ockam_core::Error::new(Origin::Core, Kind::Internal, "invalid system time")
        })?;
        if credential_data.expires <= now {
            return Ok(ProcessArrivedCredentialResult::BadRequest(
                "expired credential",
            ));
        }

        let issuer = match authorities.get(&credential_data.issuer) {
            Some(i) => i,
            None => {
                return Ok(ProcessArrivedCredentialResult::BadRequest(
                    "unknown authority",
                ));
            }
        };

        let credential_data = match issuer.verify_credential(&credential, vault).await {
            Ok(d) => d,
            Err(_) => {
                return Ok(ProcessArrivedCredentialResult::BadRequest(
                    "credential verification failed",
                ))
            }
        };

        // TODO: Implement expiration mechanism in Storage
        let entry = AttributesEntry::new(credential_data.attributes, credential_data.expires);

        let entry = minicbor::to_vec(&entry)?;

        authenticated_storage
            .set(
                &sender.to_string(),
                IdentityStateConst::ATTRIBUTES_KEY.to_string(),
                entry,
            )
            .await?;

        Ok(ProcessArrivedCredentialResult::Ok())
    }

    /// Return authenticated non-expired attributes attached to that Identity
    pub async fn get_attributes(
        identity_id: &IdentityIdentifier,
        authenticated_storage: &impl AuthenticatedStorage,
    ) -> Result<Option<BTreeMap<String, Vec<u8>>>> {
        let id = identity_id.to_string();
        let entry = match authenticated_storage
            .get(&id, IdentityStateConst::ATTRIBUTES_KEY)
            .await?
        {
            Some(e) => e,
            None => return Ok(None),
        };

        let entry: AttributesEntry = minicbor::decode(&entry)?;

        let now = Timestamp::now().ok_or_else(|| {
            ockam_core::Error::new(Origin::Core, Kind::Internal, "invalid system time")
        })?;
        if entry.expires <= now {
            authenticated_storage
                .del(&id, IdentityStateConst::ATTRIBUTES_KEY)
                .await?;
            return Ok(None);
        }

        let attrs = entry.attrs().to_owned();

        Ok(Some(attrs))
    }

    /// Start worker that will be available to receive others attributes and put them into storage,
    /// after successful verification
    pub async fn start_worker(
        &self,
        authorities: BTreeMap<IdentityIdentifier, PublicIdentity>,
        address: impl Into<Address>,
        present_back: Option<Credential<'static>>,
        authenticated_storage: impl AuthenticatedStorage,
        vault: impl IdentityVault,
    ) -> Result<()> {
        let worker =
            CredentialExchangeWorker::new(authorities, present_back, authenticated_storage, vault);

        self.ctx.start_worker(address.into(), worker).await
    }

    /// Present credential to other party, route shall use secure channel
    async fn present_credential(
        &self,
        credential: Credential<'_>,
        route: impl Into<Route>,
    ) -> Result<()> {
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
            _ => Err(ockam_core::Error::new(
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
        credential: Credential<'_>,
        route: impl Into<Route>,
        authorities: &BTreeMap<IdentityIdentifier, PublicIdentity>,
        authenticated_storage: &impl AuthenticatedStorage,
        vault: &impl IdentityVault,
    ) -> Result<()> {
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
                return Err(ockam_core::Error::new(
                    Origin::Application,
                    Kind::Invalid,
                    "credential presentation failed",
                ))
            }
        }

        let credential: Credential = dec.decode()?;

        let res = Self::receive_presented_credential(
            their_id,
            credential,
            authorities,
            vault,
            authenticated_storage,
        )
        .await?;

        match res {
            ProcessArrivedCredentialResult::Ok() => Ok(()),
            ProcessArrivedCredentialResult::BadRequest(str) => Err(ockam_core::Error::new(
                Origin::Application,
                Kind::Protocol,
                str,
            )),
        }
    }
}

/// Worker responsible for receiving and verifying other party's credentials
pub struct CredentialExchangeWorker<S: AuthenticatedStorage, V: IdentityVault> {
    authorities: BTreeMap<IdentityIdentifier, PublicIdentity>,
    present_back: Option<Credential<'static>>,
    authenticated_storage: S,
    vault: V,
}

impl<S: AuthenticatedStorage, V: IdentityVault> CredentialExchangeWorker<S, V> {
    pub fn new(
        authorities: BTreeMap<IdentityIdentifier, PublicIdentity>,
        present_back: Option<Credential<'static>>,
        authenticated_storage: S,
        vault: V,
    ) -> Self {
        Self {
            authorities,
            present_back,
            authenticated_storage,
            vault,
        }
    }
}

impl<S: AuthenticatedStorage, V: IdentityVault> CredentialExchangeWorker<S, V> {
    async fn handle_request(
        &mut self,
        _ctx: &mut Context,
        req: &Request<'_>,
        sender: IdentityIdentifier,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        trace! {
            target: TARGET,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use ockam_core::api::Method::*;
        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => {
                return Ok(Response::bad_request(req.id())
                    .body("Invalid method")
                    .to_vec()?)
            }
        };

        let r = match (method, path_segments.as_slice()) {
            (Post, ["actions", "present"]) => {
                let credential: Credential = dec.decode()?;

                let res = CredentialExchange::receive_presented_credential(
                    sender,
                    credential,
                    &self.authorities,
                    &self.vault,
                    &self.authenticated_storage,
                )
                .await?;

                match res {
                    ProcessArrivedCredentialResult::Ok() => Response::ok(req.id()).to_vec()?,
                    ProcessArrivedCredentialResult::BadRequest(str) => {
                        CredentialExchange::bad_request(req.id(), req.path(), str).to_vec()?
                    }
                }
            }
            (Post, ["actions", "present_mutual"]) => {
                let credential: Credential = dec.decode()?;

                let res = CredentialExchange::receive_presented_credential(
                    sender,
                    credential,
                    &self.authorities,
                    &self.vault,
                    &self.authenticated_storage,
                )
                .await?;

                if let ProcessArrivedCredentialResult::BadRequest(str) = res {
                    CredentialExchange::bad_request(req.id(), req.path(), str).to_vec()?
                } else {
                    match &self.present_back {
                        Some(p) => Response::ok(req.id()).body(p).to_vec()?,
                        None => Response::ok(req.id()).to_vec()?,
                    }
                }
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req.id())
                    .body(format!("Invalid endpoint: {}", path))
                    .to_vec()?
            }
        };
        Ok(r)
    }
}

#[async_trait]
impl<S: AuthenticatedStorage, V: IdentityVault> Worker for CredentialExchangeWorker<S, V> {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let sender = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?
            .their_identity_id()
            .clone();

        let r = match self.handle_request(ctx, &req, sender, &mut dec).await {
            Ok(r) => r,
            // If an error occurs, send a response with the error code so the listener can
            // fail fast instead of failing silently here and force the listener to timeout.
            Err(err) => {
                error!(?err, "Failed to handle message");
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err.to_string())
                    .to_vec()?
            }
        };
        ctx.send(msg.return_route(), r).await
    }
}

#[cfg(test)]
mod tests {
    use crate::authenticated_storage::mem::InMemoryStorage;
    use crate::authenticated_storage::AuthenticatedStorage;
    use crate::credential::{
        Credential, CredentialBuilder, CredentialExchange, CredentialExchangeWorker,
    };
    use crate::{
        Identity, IdentityIdentifier, IdentityStateConst, PublicIdentity, TrustEveryonePolicy,
        TrustIdentifierPolicy,
    };
    use ockam_core::{route, Result};
    use ockam_node::Context;
    use ockam_vault::Vault;
    use std::collections::BTreeMap;
    use std::time::Duration;

    #[ockam_macros::test]
    async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let authority = Identity::create(ctx, &vault).await?;

        let credential_exchange = CredentialExchange::create(ctx).await?;

        let server = Identity::create(ctx, &vault).await?;
        let server_storage = InMemoryStorage::new();

        server
            .create_secure_channel_listener("listener", TrustEveryonePolicy, &server_storage)
            .await?;

        let mut authorities = BTreeMap::new();
        authorities.insert(authority.identifier().clone(), authority.to_public().await?);
        credential_exchange
            .start_worker(
                authorities,
                "credential_exchange",
                None,
                server_storage.clone(),
                vault.clone(),
            )
            .await?;

        let client = Identity::create(ctx, &vault).await?;
        let client_storage = InMemoryStorage::new();
        let channel = client
            .create_secure_channel(
                route!["listener"],
                TrustIdentifierPolicy::new(server.identifier().clone()),
                &client_storage,
            )
            .await?;

        let credential_builder = Credential::builder(client.identifier().clone());
        let credential = credential_builder.with_attribute("is_superuser", b"true");

        let credential = authority.issue_credential(credential).await?;

        credential_exchange
            .present_credential(credential, route![channel, "credential_exchange"])
            .await?;

        let attrs = CredentialExchange::get_attributes(client.identifier(), &server_storage)
            .await?
            .unwrap();

        let val = attrs.get("is_superuser").unwrap();

        assert_eq!(val.as_slice(), b"true");

        ctx.stop().await
    }

    #[ockam_macros::test]
    async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let authority = Identity::create(ctx, &vault).await?;

        let credential_exchange = CredentialExchange::create(ctx).await?;

        let client2 = Identity::create(ctx, &vault).await?;
        let client2_storage = InMemoryStorage::new();

        let credential2 =
            Credential::builder(client2.identifier().clone()).with_attribute("is_admin", b"true");

        let credential2 = authority.issue_credential(credential2).await?;

        client2
            .create_secure_channel_listener("listener", TrustEveryonePolicy, &client2_storage)
            .await?;

        let mut authorities = BTreeMap::new();
        authorities.insert(authority.identifier().clone(), authority.to_public().await?);
        credential_exchange
            .start_worker(
                authorities.clone(),
                "credential_exchange",
                Some(credential2),
                client2_storage.clone(),
                vault.clone(),
            )
            .await?;

        let client1 = Identity::create(ctx, &vault).await?;
        let client1_storage = InMemoryStorage::new();

        let credential1 =
            Credential::builder(client1.identifier().clone()).with_attribute("is_user", b"true");

        let credential1 = authority.issue_credential(credential1).await?;

        let channel = client1
            .create_secure_channel(route!["listener"], TrustEveryonePolicy, &client1_storage)
            .await?;

        credential_exchange
            .present_credential_mutual(
                credential1,
                route![channel, "credential_exchange"],
                &authorities,
                &client1_storage,
                &vault,
            )
            .await?;

        let attrs1 = CredentialExchange::get_attributes(client1.identifier(), &client2_storage)
            .await?
            .unwrap();

        assert_eq!(attrs1.get("is_user").unwrap().as_slice(), b"true");

        let attrs2 = CredentialExchange::get_attributes(client2.identifier(), &client1_storage)
            .await?
            .unwrap();

        assert_eq!(attrs2.get("is_admin").unwrap().as_slice(), b"true");

        ctx.stop().await
    }
}
