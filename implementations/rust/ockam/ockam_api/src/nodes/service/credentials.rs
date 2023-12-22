use std::str::FromStr;

use miette::IntoDiagnostic;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::Result;
use ockam_core::api::{Error, Request, Response};
use ockam_core::async_trait;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::cloud::AuthorityNodeClient;
use crate::local_multiaddr_to_route;
use crate::nodes::models::credentials::{GetCredentialRequest, PresentCredentialRequest};
use crate::nodes::{BackgroundNodeClient, NodeManager};

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
impl Credentials for AuthorityNodeClient {
    async fn get_credential(
        &self,
        ctx: &Context,
        overwrite: bool,
        identity_name: Option<String>,
    ) -> miette::Result<CredentialAndPurposeKey> {
        let body = GetCredentialRequest::new(overwrite, identity_name);
        let req = Request::post("/node/credentials/actions/get").body(body);
        self.secure_client
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
        self.secure_client
            .tell(ctx, "", req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }
}

#[async_trait]
impl Credentials for BackgroundNodeClient {
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
        ctx: &Context,
        request: GetCredentialRequest,
    ) -> Result<Response<CredentialAndPurposeKey>, Response<Error>> {
        match self
            .node_manager
            .get_credential_by_identity_name(ctx, request.identity_name.clone(), None)
            .await
        {
            Ok(Some(c)) => Ok(Response::ok().body(c)),
            Ok(None) => Err(Response::not_found_no_request(&format!(
                "no credential found for {}",
                request
                    .identity_name
                    .unwrap_or("default identity".to_string())
            ))),
            Err(e) => Err(Response::internal_error_no_request(&format!(
                "Error retrieving credential from authority for {}: {}",
                request
                    .identity_name
                    .unwrap_or("default identity".to_string()),
                e,
            ))),
        }
    }

    pub(super) async fn present_credential(
        &self,
        ctx: &Context,
        request: PresentCredentialRequest,
    ) -> Result<Response, Response<Error>> {
        // TODO: Replace with self.connect?
        let route = match MultiAddr::from_str(&request.route) {
            Ok(route) => route,
            Err(e) => {
                return Err(Response::bad_request_no_request(&format!(
                    "Couldn't convert {} to a MultiAddr: {e:?}",
                    request.route
                )))
            }
        };

        match self
            .node_manager
            .present_credential(ctx, route, request.oneway)
            .await
        {
            Ok(()) => Ok(Response::ok()),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }
}

impl NodeManager {
    pub(super) async fn present_credential(
        &self,
        ctx: &Context,
        route: MultiAddr,
        oneway: bool,
    ) -> Result<()> {
        let route = local_multiaddr_to_route(&route)?;
        let credential = self
            .get_credential(ctx, &self.identifier(), None)
            .await?
            .unwrap_or_else(|| panic!("A credential must be retrieved for {}", self.identifier()));

        if oneway {
            self.credentials_service()
                .present_credential(ctx, route, credential)
                .await?;
        } else {
            self.credentials_service()
                .present_credential_mutual(
                    ctx,
                    route,
                    &self.trust_context()?.authorities(),
                    credential,
                )
                .await?;
        }
        Ok(())
    }
}
