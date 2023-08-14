pub mod types;

use crate::verifier::types::{VerifyRequest, VerifyResponse};
use either::Either;
use minicbor::Decoder;
use ockam::identity::models::{CredentialAndPurposeKey, PurposeKeyAttestationData, VersionedData};
use ockam::identity::{CredentialAndPurposeKeyData, Identities};
use ockam_core::api::{Error, Request, Response, ResponseBuilder};
use ockam_core::api::{Id, Method};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, api, Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

pub struct Verifier {
    identities: Arc<Identities>,
}

#[ockam_core::worker]
impl Worker for Verifier {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let r = self.on_request(m.as_body()).await?;
        c.send(m.return_route(), r).await
    }
}

impl Verifier {
    pub fn new(identities: Arc<Identities>) -> Self {
        Self { identities }
    }

    async fn on_request(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut dec = Decoder::new(data);

        let req: Request = match dec.decode() {
            Ok(rq) => rq,
            Err(e) => {
                let err = Error::default().with_message(e.to_string());
                return Ok(Response::bad_request(Id::default()).body(err).to_vec()?);
            }
        };

        trace! {
            target: "ockam_api::verifier",
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        let res = match req.method() {
            Some(Method::Post) => match req.path_segments::<2>().as_slice() {
                ["verify"] => {
                    let vr: VerifyRequest = dec.decode()?;
                    let cr: CredentialAndPurposeKey = minicbor::decode(vr.credential())?;
                    match self.verify(req.id(), &vr, &cr).await {
                        Ok(Either::Left(err)) => err.to_vec()?,
                        Ok(Either::Right(dat)) => {
                            let exp = dat.credential_data.expires_at;
                            Response::ok(req.id())
                                .body(VerifyResponse::new(
                                    dat.credential_data.subject_attributes,
                                    exp,
                                ))
                                .to_vec()?
                        }
                        Err(err) => {
                            let err_body = Error::new(req.path())
                                .with_message(format!("Unable to verify credential. {}", err));
                            Response::internal_error(req.id()).body(err_body).to_vec()?
                        }
                    }
                }
                _ => api::unknown_path(&req).to_vec()?,
            },
            _ => api::invalid_method(&req).to_vec()?,
        };

        Ok(res)
    }

    async fn verify<'a>(
        &self,
        id: Id,
        req: &'a VerifyRequest<'a>,
        cre: &CredentialAndPurposeKey,
    ) -> Result<Either<ResponseBuilder<Error>, CredentialAndPurposeKeyData>> {
        let versioned_data: VersionedData =
            minicbor::decode(cre.purpose_key_attestation.data.as_slice())?;
        let data: PurposeKeyAttestationData = minicbor::decode(&versioned_data.data)?;

        let authority = if let Some(ident) = req.authority(&data.subject) {
            // FIXME: Put Authority separately to avoid decoding the data manually here
            self.identities
                .identities_creation()
                .import(Some(&data.subject), ident)
                .await?
                .identifier()
                .clone()
        } else {
            let err = Error::new("/verify").with_message("unauthorised issuer");
            return Ok(Either::Left(Response::unauthorized(id).body(err)));
        };

        let data = match self
            .identities
            .credentials()
            .verify_credential(Some(req.subject()), &[authority], &cre)
            .await
        {
            Ok(data) => data,
            Err(err) => {
                let err = Error::new("/verify")
                    .with_message(format!("error verifying a credential: {err}"));
                return Ok(Either::Left(Response::forbidden(id).body(err)));
            }
        };

        Ok(Either::Right(data))
    }
}
