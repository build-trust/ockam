use crate::identity::models::*;
use crate::nodes::service::NodeIdentities;
use core::convert::Infallible;
use minicbor::encode::Write;
use minicbor::{Decoder, Encode};
use ockam::identity::IdentityHistoryComparison;
use ockam_core::api::{Error, Id, Method, Request, Response, Status};
use ockam_core::vault::Signature;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

/// Vault Service Worker
pub struct IdentityService {
    node_identities: NodeIdentities,
}

impl IdentityService {
    pub async fn new(node_identities: NodeIdentities) -> Result<Self> {
        Ok(Self { node_identities })
    }
}

impl IdentityService {
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
            Get => match req.path_segments::<2>().as_slice() {
                [identity_name] => {
                    match self
                        .node_identities
                        .get_identity(identity_name.to_string(), None)
                        .await?
                    {
                        Some(identity) => {
                            let body = CreateResponse::new(
                                identity.export()?,
                                identity.identifier().to_string(),
                            );
                            Self::ok_response(req, Some(body), enc)
                        }
                        None => Self::response_for_bad_request(req, "unknown identity", enc),
                    }
                }
                _ => Self::response_for_bad_request(req, "unknown path", enc),
            },
            Post => match req.path_segments::<2>().as_slice() {
                [""] => {
                    let identity = self
                        .node_identities
                        .get_default_identities_creation()
                        .await?
                        .create_identity()
                        .await?;
                    let body =
                        CreateResponse::new(identity.export()?, identity.identifier().to_string());

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "validate_identity_change_history"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<ValidateIdentityChangeHistoryRequest>()?;
                    let identities_creation = self
                        .node_identities
                        .get_default_identities_creation()
                        .await?;
                    let identity = identities_creation.import_identity(args.identity()).await?;

                    let body = ValidateIdentityChangeHistoryResponse::new(String::from(
                        identity.identifier(),
                    ));

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "create_signature"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<CreateSignatureRequest>()?;
                    let identities_creation = self
                        .node_identities
                        .get_identities_creation(args.vault_name())
                        .await?;
                    let identity = identities_creation.import_identity(args.identity()).await?;
                    let identities_keys = self
                        .node_identities
                        .get_identities_keys(args.vault_name())
                        .await?;
                    let signature = identities_keys
                        .create_signature(&identity, args.data(), None)
                        .await?;

                    let body = CreateSignatureResponse::new(signature.as_ref());

                    Self::ok_response(req, Some(body), enc)
                }
                ["actions", "verify_signature"] => {
                    if !req.has_body() {
                        return Self::response_for_bad_request(req, "empty body", enc);
                    }

                    let args = dec.decode::<VerifySignatureRequest>()?;
                    let identities_creation = self
                        .node_identities
                        .get_default_identities_creation()
                        .await?;
                    let peer_identity = identities_creation
                        .import_identity(args.signer_identity())
                        .await?;

                    let identities_keys =
                        self.node_identities.get_default_identities_keys().await?;
                    let verified = identities_keys
                        .verify_signature(
                            &peer_identity,
                            &Signature::new(args.signature().to_vec()),
                            args.data(),
                            None,
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

                    let identities_creation = self
                        .node_identities
                        .get_default_identities_creation()
                        .await?;

                    let current_identity = identities_creation
                        .import_identity(args.current_identity())
                        .await?;

                    let body = if args.known_identity().is_empty() {
                        IdentityHistoryComparison::Newer
                    } else {
                        let known_identity = identities_creation
                            .import_identity(args.known_identity())
                            .await?;
                        current_identity.compare(&known_identity)
                    };

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
impl Worker for IdentityService {
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
