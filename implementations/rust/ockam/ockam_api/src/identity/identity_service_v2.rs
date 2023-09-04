use core::convert::Infallible;
use ockam_core::compat::sync::Arc;

use minicbor::encode::Write;
use minicbor::{Decoder, Encode};
use ockam::identity::{IdentityHistoryComparison, Purpose};
use tracing::trace;

use crate::identity::models::{
    CompareIdentityChangeHistoryRequest, CreateResponse, ValidateIdentityChangeHistoryRequest,
    ValidateIdentityChangeHistoryResponse, CreatePurposeKeyRequest, CreatePurposeKeyResponse,
};
use ockam_core::api::{Error, Id, Method, Request, Response, Status};
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;

use ockam::identity::identities::Identities;

/// Vault Service Worker
pub struct IdentityServiceV2 {
    identities:  Arc<Identities>,
}

impl IdentityServiceV2 {
    pub async fn new(identities: Arc<Identities>) -> Result<Self> {
        Ok(Self { identities })
    }
}

impl IdentityServiceV2 {
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
        req: &Request,
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
                    let identity = self
                        .identities
                        .identities_creation()
                        .create_identity()
                        .await?;
                    debug!("Created identity: {:?}", identity);
                    let body =
                        CreateResponse::new(identity.export()?, identity.identifier().clone());

                    Self::ok_response(req, Some(body), enc)
                }
                ["purpose_key"] => {
                    debug!("purpose_key msg received");
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }
                    debug!("purpose_key about to decode");
                    let args = dec.decode::<CreatePurposeKeyRequest>()?;
                    debug!("decoded");
                    let secure_channel_key = self.identities
                        .purpose_keys()
                        .create_purpose_key(args.identity_id(), Purpose::SecureChannel).await?;
                    let body = CreatePurposeKeyResponse::new(secure_channel_key);
                    Self::ok_response(req, Some(body), enc)
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Get | Put | Patch | Delete => Self::response_for_bad_request(req, "unknown method", enc),
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
impl Worker for IdentityServiceV2 {
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
