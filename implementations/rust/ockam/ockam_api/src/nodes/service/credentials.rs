use std::str::FromStr;

use either::Either;
use miette::IntoDiagnostic;
use minicbor::Decoder;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::Result;
use ockam_core::api::{Error, Request, RequestHeader, Response};
use ockam_core::async_trait;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cli_state::traits::StateDirTrait;
use crate::cloud::AuthorityNode;
use crate::error::ApiError;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};

use super::NodeManagerWorker;

#[async_trait]
pub trait Credentials {
    async fn authenticate(
        &self,
        ctx: &Context,
        identity_name: Option<String>,
    ) -> miette::Result<()> {
        let _ = self.get_credential(ctx, false, identity_name).await?;
        Ok(())
    }

    async fn get_credential(
        &self,
        ctx: &Context,
        overwrite: bool,
        identity_name: Option<String>,
    ) -> miette::Result<CredentialAndPurposeKey>;
}

#[async_trait]
impl Credentials for AuthorityNode {
    async fn get_credential(
        &self,
        ctx: &Context,
        overwrite: bool,
        identity_name: Option<String>,
    ) -> miette::Result<CredentialAndPurposeKey> {
        let body = GetCredentialRequest::new(overwrite, identity_name);
        let req = Request::post("/node/credentials/actions/get").body(body);
        self.0
            .ask(ctx, "", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}

impl NodeManagerWorker {
    pub(super) async fn get_credential(
        &mut self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Either<Response<Error>, Response<CredentialAndPurposeKey>>> {
        let request: GetCredentialRequest = dec.decode()?;

        let identifier = if let Some(identity) = &request.identity_name {
            self.node_manager
                .cli_state
                .identities
                .get(identity)?
                .identifier()
        } else {
            self.node_manager.identifier()
        };

        match self
            .node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &identifier)
            .await
        {
            Ok(c) => Ok(Either::Right(Response::ok(req).body(c))),
            Err(e) => Ok(Either::Left(Response::internal_error(
                req,
                &format!(
                    "Error retrieving credential from authority for {}: {}",
                    identifier, e,
                ),
            ))),
        }
    }

    pub(super) async fn present_credential(
        &self,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        ctx: &Context,
    ) -> Result<Response, Response<Error>> {
        let request: PresentCredentialRequest = dec.decode()?;

        // TODO: Replace with self.connect?
        let route = MultiAddr::from_str(&request.route).map_err(|_| {
            ApiError::core(format!(
                "Couldn't convert String to MultiAddr: {}",
                &request.route
            ))
        })?;
        let route = match local_multiaddr_to_route(&route) {
            Some(route) => route,
            None => return Err(ApiError::core("Invalid credentials service route").into()),
        };

        let credential = self
            .node_manager
            .trust_context()?
            .authority()?
            .credential(ctx, &self.node_manager.identifier())
            .await?;

        if request.oneway {
            self.node_manager
                .credentials_service()
                .present_credential(ctx, route, credential)
                .await?;
        } else {
            self.node_manager
                .credentials_service()
                .present_credential_mutual(
                    ctx,
                    route,
                    self.node_manager
                        .trust_context()?
                        .authorities()
                        .await?
                        .as_slice(),
                    credential,
                )
                .await?;
        }

        let response = Response::ok(req);
        Ok(response)
    }
}
