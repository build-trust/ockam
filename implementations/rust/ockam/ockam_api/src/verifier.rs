pub mod types;

use either::Either;
use minicbor::Decoder;
use ockam_core::api::{self, Id, ResponseBuilder};
use ockam_core::api::{Error, Method, Request, Response};
use ockam_core::{self, Result, Routed, Worker};
use ockam_identity::credential::{Credential, CredentialData, Verified};
use ockam_identity::{IdentityVault, PublicIdentity};
use ockam_node::Context;
use tracing::trace;

use self::types::{VerifyRequest, VerifyResponse};

#[derive(Debug)]
pub struct Verifier<V> {
    vault: V,
}

#[ockam_core::worker]
impl<V> Worker for Verifier<V>
where
    V: IdentityVault,
{
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(&mut self, c: &mut Context, m: Routed<Self::Message>) -> Result<()> {
        let r = self.on_request(m.as_body()).await?;
        c.send(m.return_route(), r).await
    }
}

impl<V> Verifier<V>
where
    V: IdentityVault,
{
    pub fn new(vault: V) -> Self {
        Self { vault }
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
        let data = CredentialData::try_from(cre)?;

        let ident = if let Some(ident) = req.authority(data.unverfied_issuer()) {
            PublicIdentity::import(ident, &self.vault).await?
        } else {
            let err = Error::new("/verify").with_message("unauthorised issuer");
            return Ok(Either::Left(Response::unauthorized(id).body(err)));
        };

        let data = match ident
            .verify_credential(cre, req.subject(), &self.vault)
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
