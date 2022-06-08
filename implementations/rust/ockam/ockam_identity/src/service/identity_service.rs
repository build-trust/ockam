use crate::service::models::*;
use crate::{
    Contact, ExportedIdentity, Identity, IdentityIdentifier, IdentityTrait, IdentityVault,
};
use minicbor::encode::Write;
use minicbor::{Decoder, Encode};
use ockam_api::{Error, Id, Method, Request, Response, Status};
use ockam_core::compat::io;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

/// Vault Service Worker
pub struct IdentityService<V: IdentityVault> {
    ctx: Context,
    vault: V,
}

impl<V: IdentityVault> IdentityService<V> {
    pub async fn create(ctx: &Context, address: impl Into<Address>, vault: V) -> Result<()> {
        let s = Self {
            ctx: ctx.new_detached(Address::random_local()).await?,
            vault,
        };
        ctx.start_worker(address.into(), s).await
    }
}

impl<V: IdentityVault> IdentityService<V> {
    fn response_for_bad_request<W>(req: &Request, msg: &str, enc: W) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        let error = Error::new(req.path()).with_message(msg);

        let error = if let Some(m) = req.method() {
            error.with_method(m)
        } else {
            error
        };

        Response::bad_request(req.id()).body(error).encode(enc)?;

        Ok(())
    }

    fn ok_response<W, B>(req: &Request, body: Option<B>, enc: W) -> Result<()>
    where
        W: Write<Error = io::Error>,
        B: Encode<()>,
    {
        Response::ok(req.id()).body(body).encode(enc)?;

        Ok(())
    }

    fn response_with_error<W>(
        req: Option<&Request>,
        status: Status,
        error: &str,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        let (path, req_id) = match req {
            None => ("", Id::fresh()),
            Some(req) => (req.path(), req.id()),
        };

        let error = Error::new(path).with_message(error);

        Response::builder(req_id, status).body(error).encode(enc)?;

        Ok(())
    }

    async fn handle_request<W>(
        &mut self,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
        enc: W,
    ) -> Result<()>
    where
        W: Write<Error = io::Error>,
    {
        trace! {
            target: "ockam_identity::service",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let method = match req.method() {
            Some(m) => m,
            None => return Self::response_for_bad_request(req, "empty method", enc),
        };

        use Method::*;

        match method {
            Post => match req.path_segments::<2>().as_slice() {
                ["identities"] => {
                    let identity = Identity::create(&self.ctx, &self.vault).await?;
                    let identifier = identity.identifier().await?;

                    let body = CreateResponse::new(
                        identity.export().await.export()?,
                        String::from(identifier),
                    );

                    Self::ok_response(req, Some(body), enc)
                }
                ["identities", "verify_and_add_contact"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<VerifyAndAddContactRequest>()?;

                    let identity = ExportedIdentity::import(args.identity())?;
                    let identity = Identity::import(&self.ctx, &self.vault, identity).await?;
                    let contact = Contact::import(args.contact())?;
                    let contact_id = String::from(contact.identifier().clone());

                    if identity.get_contact(contact.identifier()).await?.is_none() {
                        identity.verify_and_add_contact(contact).await?;
                    } else {
                        // TODO: Support updating
                    }

                    let body = VerifyAndAddContactResponse::new(
                        identity.export().await.export()?,
                        contact_id,
                    );

                    #[allow(unused_qualifications)]
                    Self::ok_response(req, Some(body), enc)
                }
                ["identities", "create_auth_proof"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<CreateAuthProofRequest>()?;
                    let identity = ExportedIdentity::import(args.identity())?;
                    let identity = Identity::import(&self.ctx, &self.vault, identity).await?;

                    let proof = identity.create_auth_proof(args.state()).await?;

                    let body = CreateAuthProofResponse::new(proof);

                    Self::ok_response(req, Some(body), enc)
                }
                ["identities", "verify_auth_proof"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<VerifyAuthProofRequest>()?;
                    let identity = ExportedIdentity::import(args.identity())?;
                    let identity = Identity::import(&self.ctx, &self.vault, identity).await?;
                    let peer_identifier = IdentityIdentifier::try_from(args.peer_identity_id())?;

                    let verified = identity
                        .verify_auth_proof(args.state(), &peer_identifier, args.proof())
                        .await?;

                    let body = VerifyAuthProofResponse::new(verified);

                    Self::ok_response(req, Some(body), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Get => match req.path_segments::<2>().as_slice() {
                ["identities", "contact"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<ContactRequest>()?;

                    let identity = ExportedIdentity::import(args.identity())?;
                    let identity = Identity::import(&self.ctx, &self.vault, identity).await?;

                    let contact = identity.as_contact().await?.export()?;

                    let body = ContactResponse::new(contact);

                    Self::ok_response(req, Some(body), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Put | Patch | Delete => Self::response_for_bad_request(req, "unknown method", enc),
        }
    }

    async fn on_request(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        let mut dec = Decoder::new(data);
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(_) => {
                Self::response_with_error(
                    None,
                    Status::BadRequest,
                    "invalid Request structure",
                    &mut buf,
                )?;

                return Ok(buf);
            }
        };

        match self.handle_request(&req, &mut dec, &mut buf).await {
            Ok(_) => {}
            Err(err) => Self::response_with_error(
                Some(&req),
                Status::InternalServerError,
                &err.to_string(),
                &mut buf,
            )?,
        }

        Ok(buf)
    }
}

#[ockam_core::worker]
impl<V: IdentityVault> Worker for IdentityService<V> {
    type Message = Vec<u8>;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let buf = self.on_request(msg.as_body()).await?;
        ctx.send(msg.return_route(), buf).await
    }
}
