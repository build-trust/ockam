use core::convert::Infallible;

use crate::identity::models::*;
use crate::{Error, Id, Method, Request, Response, Status};
use minicbor::encode::Write;
use minicbor::{Decoder, Encode};
use ockam_core::vault::Signature;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_identity::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use ockam_identity::error::IdentityError;
use ockam_identity::{Identity, IdentityVault};
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
        W: Write<Error = Infallible>,
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
        W: Write<Error = Infallible>,
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
        W: Write<Error = Infallible>,
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
        W: Write<Error = Infallible>,
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
                [""] => {
                    let identity = Identity::create(&self.ctx, &self.vault).await?;
                    let identifier = identity.identifier()?;

                    let body =
                        CreateResponse::new(identity.export().await?, String::from(identifier));

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "validate_identity_change_history"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<ValidateIdentityChangeHistoryRequest>()?;
                    let identity =
                        Identity::import(&self.ctx, args.identity(), &self.vault).await?;

                    let body = ValidateIdentityChangeHistoryResponse::new(String::from(
                        identity.identifier()?,
                    ));

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "create_signature"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<CreateSignatureRequest>()?;
                    let identity =
                        Identity::import(&self.ctx, args.identity(), &self.vault).await?;

                    let signature = identity.create_signature(args.data()).await?;

                    let body = CreateSignatureResponse::new(signature.as_ref());

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "verify_signature"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<VerifySignatureRequest>()?;
                    let peer_identity = IdentityChangeHistory::import(args.signer_identity())?;
                    if !peer_identity
                        .verify_all_existing_events(&self.vault)
                        .await?
                    {
                        return Err(IdentityError::ConsistencyError.into());
                    }
                    let public_key = peer_identity.get_first_root_public_key()?;

                    let verified = self
                        .vault
                        .verify(
                            &Signature::new(args.signature().to_vec()),
                            &public_key,
                            args.data(),
                        )
                        .await?;

                    let body = VerifySignatureResponse::new(verified);

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "compare_identity_change_history"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<CompareIdentityChangeHistoryRequest>()?;

                    let current_identity = IdentityChangeHistory::import(args.current_identity())?;
                    if !current_identity
                        .verify_all_existing_events(&self.vault)
                        .await?
                    {
                        return Err(IdentityError::ConsistencyError.into());
                    }

                    let body = if args.known_identity().is_empty() {
                        IdentityHistoryComparison::Newer
                    } else {
                        let known_identity = IdentityChangeHistory::import(args.known_identity())?;
                        if !known_identity
                            .verify_all_existing_events(&self.vault)
                            .await?
                        {
                            return Err(IdentityError::ConsistencyError.into());
                        }

                        current_identity.compare(&known_identity)
                    };

                    Self::ok_response(req, Some(body), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Get | Put | Patch | Delete => {
                Self::response_for_bad_request(req, "unknown method", enc)
            }
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
