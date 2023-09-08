use std::path::PathBuf;

use anyhow::Context as _;
use miette::miette;
use minicbor::{Decode, Encode};
use tracing::info;

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::CredentialsIssuerClient;
use ockam::identity::OneTimeCode;
use ockam::Context;
use ockam_api::cli_state::{ProjectConfigCompact, StateDirTrait, StateItemTrait};
use ockam_api::{
    config::lookup::ProjectLookup, nodes::models::secure_channel::CredentialExchangeMode,
    DefaultAddress,
};
use ockam_core::api::RequestBuilder;
use ockam_core::route;
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;

use crate::node::util::delete_embedded_node;
use crate::{
    node::util::{delete_node, start_embedded_node_with_vault_and_identity},
    project::util::{create_secure_channel_to_authority, create_secure_channel_to_project},
    util::Rpc,
    CommandGlobalOpts, Result,
};

use super::api::TrustContextOpts;

pub enum OrchestratorEndpoint {
    Authenticator,
    Project,
}

/// Helps build an Orchestrator API Request
pub struct OrchestratorApiBuilder<'a> {
    ctx: &'a Context,
    opts: &'a CommandGlobalOpts,
    trust_context_opts: &'a TrustContextOpts,
    node_name: Option<String>,
    destination: OrchestratorEndpoint,
    identity: Option<String>,
    credential_exchange_mode: CredentialExchangeMode,
    project_lookup: Option<ProjectLookup>,
    one_time_code: Option<OneTimeCode>,
}

impl<'a> Drop for OrchestratorApiBuilder<'a> {
    fn drop(&mut self) {
        if let Some(node_name) = &self.node_name {
            let _ = delete_node(self.opts, node_name, false);
        }
    }
}

impl<'a> OrchestratorApiBuilder<'a> {
    pub fn new(
        ctx: &'a Context,
        opts: &'a CommandGlobalOpts,
        trust_context_opts: &'a TrustContextOpts,
    ) -> Self {
        OrchestratorApiBuilder {
            ctx,
            opts,
            trust_context_opts,
            node_name: None,
            destination: OrchestratorEndpoint::Project,
            identity: None,
            credential_exchange_mode: CredentialExchangeMode::Oneway,
            project_lookup: None,
            one_time_code: None,
        }
    }

    /// Creates a new embedded node to communicate with the cloud
    /// FIXME: There is an ordering issue, is as_identity/1 is used,
    ///        it *must* be called before with_new_embedded_node/1,
    ///        as the identity must be know at the time of this call.
    pub async fn with_new_embedded_node(&mut self) -> Result<&mut OrchestratorApiBuilder<'a>> {
        // TODO: always use the default vault
        let node_manager = start_embedded_node_with_vault_and_identity(
            self.ctx,
            &self.opts.state,
            None,
            self.identity.clone(),
            Some(self.trust_context_opts),
        )
        .await?;
        self.node_name = Some(node_manager.node_name());
        Ok(self)
    }

    /// Creates and sets a project lookup from a Project Info file
    pub async fn with_project_from_file(
        &mut self,
        file_path: &PathBuf,
    ) -> Result<&mut OrchestratorApiBuilder<'a>> {
        // Read (okta and authority) project parameters from project.json
        let s = tokio::fs::read_to_string(file_path).await?;
        let proj_info: ProjectConfigCompact = serde_json::from_str(&s)?;
        let project_lookup = ProjectLookup::from_project(&(&proj_info).into()).await?;

        self.project_lookup = Some(project_lookup);
        Ok(self)
    }

    /// Sets the API project look up
    // TODO: oa: will be used within the enroll flow
    #[allow(dead_code)]
    pub async fn with_project_from_lookup(
        &mut self,
        proj_name: &str,
    ) -> Result<&mut OrchestratorApiBuilder<'a>> {
        let project = self.opts.state.projects.get(proj_name)?;

        self.project_lookup = Some(ProjectLookup::from_project(project.config()).await?);
        Ok(self)
    }

    // TODO oa: will be used within enroll & auth flow
    #[allow(dead_code)]
    pub fn with_endpoint(&mut self, destination: OrchestratorEndpoint) -> &mut Self {
        self.destination = destination;
        self
    }

    // TODO oa: will be used within enroll flow
    #[allow(dead_code)]
    pub fn with_credential_exchange(&mut self, cem: CredentialExchangeMode) -> &mut Self {
        self.credential_exchange_mode = cem;
        self
    }

    pub fn as_identity(&mut self, identity: String) -> &mut Self {
        self.identity = Some(identity);
        self
    }

    // TODO oa: will be used within auth flow
    #[allow(dead_code)]
    pub fn use_one_time_code(&mut self, otc: OneTimeCode) -> &mut Self {
        self.one_time_code = Some(otc);
        self
    }

    pub async fn authenticate(&self) -> Result<CredentialAndPurposeKey> {
        let sc_addr = self
            .secure_channel_to(&OrchestratorEndpoint::Authenticator)
            .await?;

        let authenticator_route = {
            let service = MultiAddr::try_from(
                format!("/service/{}", DefaultAddress::CREDENTIAL_ISSUER).as_str(),
            )?;
            let addr = sc_addr.concat(&service)?;
            ockam_api::local_multiaddr_to_route(&addr).ok_or(miette!("Invalid MultiAddr {addr}"))?
        };

        let client = CredentialsIssuerClient::new(
            route![DefaultAddress::RPC_PROXY, authenticator_route],
            self.ctx,
        )
        .await?;

        let credential = client.credential().await?;

        Ok(credential)
    }

    /// Sends the request and returns  the response
    pub async fn build(&mut self, service_address: &MultiAddr) -> Result<OrchestratorApi> {
        self.retrieve_project_info().await?;
        // Authenticate with the project authority node
        let _ = self.authenticate().await?;

        //  Establish a secure channel
        let sc_addr = self.secure_channel_to(&self.destination).await?;

        let mut to = sc_addr.concat(service_address)?;
        info!(
            "creating an rpc client to service: {} over secure channel {}",
            service_address, to
        );

        to.push_front(Service::new(DefaultAddress::RPC_PROXY))?;

        let node_name = self.node_name.as_ref().ok_or(miette!("Node is required"))?;
        let mut rpc = Rpc::background(self.ctx, self.opts, node_name).await?;
        rpc.set_to(&to)?;
        Ok(OrchestratorApi { rpc })
    }

    async fn retrieve_project_info(&mut self) -> Result<()> {
        if self.project_lookup.is_some() {
            return Ok(());
        }

        let project_path = match &self.trust_context_opts.project_path {
            Some(p) => p.clone(),
            None => {
                let default_project = self
                    .opts
                    .state
                    .projects
                    .default()
                    .context("A default project or project parameter is required")?;

                default_project.path().clone()
            }
        };

        self.with_project_from_file(&project_path).await?;

        Ok(())
    }

    async fn secure_channel_to(&self, endpoint: &OrchestratorEndpoint) -> Result<MultiAddr> {
        let node_name = self.node_name.as_ref().ok_or(miette!("Node is required"))?;
        let project = self
            .project_lookup
            .as_ref()
            .ok_or(miette!("Project is required"))?;
        let mut rpc = Rpc::background(self.ctx, self.opts, node_name).await?;

        let sc_addr = match endpoint {
            OrchestratorEndpoint::Authenticator => {
                let authority = project
                    .authority
                    .as_ref()
                    .ok_or(miette!("Project Authority is required"))?;
                // TODO: When we --project-path is fully deprecated
                // use the trust context authority here
                create_secure_channel_to_authority(
                    &mut rpc,
                    authority.identity_id().clone(),
                    authority.address(),
                    self.identity.clone(),
                )
                .await?
            }
            OrchestratorEndpoint::Project => {
                let project_identity = project
                    .identity_id
                    .as_ref()
                    .ok_or(miette!("Project should have identity set"))?;
                let project_route = project
                    .node_route
                    .as_ref()
                    .ok_or(miette!("Invalid project node route"))?;

                create_secure_channel_to_project(
                    &mut rpc,
                    project_route,
                    &project_identity.to_string(),
                    self.credential_exchange_mode,
                    self.identity.clone(),
                )
                .await?
            }
        };
        delete_embedded_node(self.opts, rpc.node_name()).await;
        Ok(sc_addr)
    }
}

pub struct OrchestratorApi {
    rpc: Rpc,
}

impl OrchestratorApi {
    pub async fn ask<T, R>(&mut self, req: RequestBuilder<T>) -> Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        let response = self.rpc.ask(req).await?;
        info!("Response is OK!");
        Ok(response)
    }

    pub async fn tell<T>(&mut self, req: RequestBuilder<T>) -> Result<()>
    where
        T: Encode<()>,
    {
        info!("Initializing request to orchestrator");
        self.rpc.tell(req).await
    }
}
