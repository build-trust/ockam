use crate::credential::{Attributes, Credential, CredentialData, Timestamp, Unverified};
use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_channel::SecureChannelLocalInfo;
use ockam_core::api::Method::Post;
use ockam_core::api::{Error, Request, Response, ResponseBuilder, Status};
use ockam_core::compat::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Result, Route, Routed, Worker};
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::change_history::IdentityChangeHistory;
use ockam_identity::error::IdentityError;
use ockam_identity::{
    IdentityIdentifier, IdentitySecureChannelLocalInfo, IdentityStateConst, IdentityVault,
};
use ockam_node::api::request;
use ockam_node::Context;

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

pub struct CredentialExchange {
    ctx: Context,
}

impl CredentialExchange {
    pub async fn create(ctx: &Context) -> Result<Self> {
        Ok(Self {
            ctx: ctx.new_detached(Address::random_local()).await?,
        })
    }

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

        let entry: AttributesEntry = Decoder::new(&entry).decode()?;

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

    pub async fn start_worker(
        &self,
        authorities: BTreeMap<IdentityIdentifier, IdentityChangeHistory>,
        address: impl Into<Address>,
        authenticated_storage: impl AuthenticatedStorage,
        vault: impl IdentityVault,
    ) -> Result<()> {
        let worker = CredentialExchangeWorker::new(authorities, authenticated_storage, vault);

        self.ctx.start_worker(address.into(), worker).await
    }

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

        let mut dec = Decoder::new(&buf);
        let res: Response = dec.decode()?;
        match res.status() {
            Some(Status::Ok) => Ok(()),
            _ => Err(ockam_core::Error::new(
                Origin::Application,
                Kind::Invalid,
                "credential presentation failed",
            )),
        }
    }
}

/// Worker responsible for receiving and verifying other party's credentials
pub struct CredentialExchangeWorker<S: AuthenticatedStorage, V: IdentityVault> {
    authorities: BTreeMap<IdentityIdentifier, IdentityChangeHistory>,
    authenticated_storage: S,
    vault: V,
}

impl<S: AuthenticatedStorage, V: IdentityVault> CredentialExchangeWorker<S, V> {
    pub fn new(
        authorities: BTreeMap<IdentityIdentifier, IdentityChangeHistory>,
        authenticated_storage: S,
        vault: V,
    ) -> Self {
        Self {
            authorities,
            authenticated_storage,
            vault,
        }
    }

    async fn receive_presented_credential(
        &self,
        req: &Request<'_>,
        sender: IdentityIdentifier,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let credential: Credential = dec.decode()?;

        let mut decoder = Decoder::new(&credential.data);
        let credential_data: CredentialData<Unverified> = decoder.decode()?;

        if credential_data.subject != sender {
            return Ok(ockam_core::api::bad_request(req, "unknown authority").to_vec()?);
        }

        let now = Timestamp::now().ok_or_else(|| {
            ockam_core::Error::new(Origin::Core, Kind::Internal, "invalid system time")
        })?;
        if credential_data.expires <= now {
            return Ok(ockam_core::api::bad_request(req, "expired credential").to_vec()?);
        }

        let issuer = match self.authorities.get(&credential_data.issuer) {
            Some(i) => i,
            None => {
                return Ok(ockam_core::api::bad_request(req, "unknown authority").to_vec()?);
            }
        };

        let credential_data = match credential.verify_signature(issuer, &self.vault).await {
            Ok(d) => d,
            Err(_) => {
                return Ok(
                    ockam_core::api::bad_request(req, "credential verification failed").to_vec()?,
                )
            }
        };

        // TODO: Implement expiration mechanism in Storage
        let entry = AttributesEntry::new(credential_data.attributes, credential_data.expires);

        let mut entry_buf = vec![];
        let mut encoder = Encoder::new(&mut entry_buf);
        encoder.encode(entry)?;

        self.authenticated_storage
            .set(
                &sender.to_string(),
                IdentityStateConst::ATTRIBUTES_KEY.to_string(),
                entry_buf,
            )
            .await?;

        Ok(Response::ok(req.id()).to_vec()?)
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
                self.receive_presented_credential(req, sender, dec).await?
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

#[crate::worker]
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
    use crate::credential::{
        Credential, CredentialBuilder, CredentialExchange, CredentialExchangeWorker,
    };
    use minicbor::Encoder;
    use ockam_core::{route, Result};
    use ockam_identity::authenticated_storage::mem::InMemoryStorage;
    use ockam_identity::authenticated_storage::AuthenticatedStorage;
    use ockam_identity::change_history::IdentityChangeHistory;
    use ockam_identity::{
        Identity, IdentityIdentifier, IdentityStateConst, TrustEveryonePolicy,
        TrustIdentifierPolicy,
    };
    use ockam_node::Context;
    use ockam_vault::Vault;
    use std::collections::BTreeMap;
    use std::time::Duration;

    #[allow(non_snake_case)]
    #[ockam_macros::test]
    async fn full_flow(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let authority = Identity::create(ctx, &vault).await?;

        let credential_exchange = CredentialExchange::create(ctx).await?;

        let server = Identity::create(ctx, &vault).await?;
        let server_storage = InMemoryStorage::new();

        server
            .create_secure_channel_listener("listener", TrustEveryonePolicy, &server_storage)
            .await?;

        let mut authorities = BTreeMap::<IdentityIdentifier, IdentityChangeHistory>::new();
        authorities.insert(authority.identifier().clone(), authority.changes().await?);
        credential_exchange
            .start_worker(
                authorities,
                "credential_exchange",
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
        let credential = credential_builder
            .with_attribute("is_superuser", b"true")
            .issue(&authority)
            .await?;

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
}
