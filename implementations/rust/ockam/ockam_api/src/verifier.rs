pub mod types;

use either::Either;
use minicbor::Decoder;
use ockam::identity::credential::{Credential, CredentialData, Verified};
use ockam::identity::Identities;
use ockam_core::api::{self, Id, ResponseBuilder};
use ockam_core::api::{Error, Method, Request, Response};
use ockam_core::compat::sync::Arc;
use ockam_core::{self, Result, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

use self::types::{VerifyRequest, VerifyResponse};

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
                    let cr: Credential = minicbor::decode(vr.credential())?;
                    match self.verify(req.id(), &vr, &cr).await {
                        Ok(Either::Left(err)) => err.to_vec()?,
                        Ok(Either::Right(dat)) => {
                            let exp = dat.expires_at();
                            Response::ok(req.id())
                                .body(VerifyResponse::new(dat.into_attributes(), exp))
                                .to_vec()?
                        }
                        Err(err) => Response::internal_error(req.id())
                            .body(err.to_string())
                            .to_vec()?,
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
        cre: &Credential,
    ) -> Result<Either<ResponseBuilder<Error<'_>>, CredentialData<Verified>>> {
        let data = CredentialData::try_from(cre.data.as_slice())?;

        let authority = if let Some(ident) = req.authority(data.unverified_issuer()) {
            self.identities
                .identities_creation()
                .import_identity(ident)
                .await?
        } else {
            let err = Error::new("/verify").with_message("unauthorised issuer");
            return Ok(Either::Left(Response::unauthorized(id).body(err)));
        };

        let data = match self
            .identities
            .credentials()
            .verify_credential(req.subject(), &[authority], cre.clone())
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
