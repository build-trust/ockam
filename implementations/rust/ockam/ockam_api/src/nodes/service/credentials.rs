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
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::BackgroundNode;

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

    async fn present_credential(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        oneway: bool,
    ) -> miette::Result<()>;
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

    async fn present_credential(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        oneway: bool,
    ) -> miette::Result<()> {
        let body = PresentCredentialRequest::new(to, oneway);
        let req = Request::post("/node/credentials/actions/present").body(body);
        self.0
            .tell(ctx, "", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}

#[async_trait]
impl Credentials for BackgroundNode {
    async fn get_credential(
        &self,
        ctx: &Context,
        overwrite: bool,
        identity_name: Option<String>,
    ) -> miette::Result<CredentialAndPurposeKey> {
        let body = GetCredentialRequest::new(overwrite, identity_name);
        self.ask(
            ctx,
            Request::post("/node/credentials/actions/get").body(body),
        )
        .await
    }

    async fn present_credential(
        &self,
        ctx: &Context,
        to: &MultiAddr,
        oneway: bool,
    ) -> miette::Result<()> {
        let body = PresentCredentialRequest::new(to, oneway);
        self.tell(
            ctx,
            Request::post("/node/credentials/actions/present").body(body),
        )
        .await
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
            self.node_manager.identifier().clone()
        };

        let credential_retriever =
            if let Some(credential_retriever) = self.node_manager.credential_retriever.as_ref() {
                credential_retriever
            } else {
                return Ok(Either::Left(Response::internal_error(
                    req,
                    &format!(
                        "Error retrieving credential for {}: No Retriever",
                        identifier,
                    ),
                )));
            };

        match credential_retriever.retrieve(ctx, &identifier).await {
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
}
