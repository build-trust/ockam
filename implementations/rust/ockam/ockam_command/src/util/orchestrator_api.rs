use std::path::PathBuf;

use crate::{
    node::util::{delete_node, start_embedded_node_with_vault_and_identity},
    project::{
        util::{create_secure_channel_to_authority, create_secure_channel_to_project},
        ProjectInfo,
    },
    util::Rpc,
    CommandGlobalOpts,
};
use anyhow::{Context as _, Result};
use minicbor::{Decode, Encode};
use ockam::Context;
use ockam_api::{
    authenticator::direct::{types::OneTimeCode, Client},
    config::lookup::ProjectLookup,
    nodes::models::secure_channel::CredentialExchangeMode,
    DefaultAddress,
};
use ockam_core::api::RequestBuilder;
use ockam_identity::credential::Credential;
use ockam_multiaddr::MultiAddr;
use tracing::info;

use super::{api::ProjectOpts, RpcBuilder};

pub enum OrchestratorEndpoint {
    Authenticator,
    Project,
}
/// Helps build an Orchestrator API Request
pub struct OrchestratorApiBuilder<'a> {
    ctx: &'a Context,
    opts: &'a CommandGlobalOpts,
    project_opts: &'a ProjectOpts,
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
        project_opts: &'a ProjectOpts,
    ) -> Self {
        OrchestratorApiBuilder {
            ctx,
            opts,
            project_opts,
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
    ///        it *must* be called before with_new_embbeded_node/1,
    ///        as the identity must be know at the time of this call.
    pub async fn with_new_embbeded_node(&mut self) -> Result<&mut OrchestratorApiBuilder<'a>> {
        // TODO: always use the default vault
        let node_name = start_embedded_node_with_vault_and_identity(
            self.ctx,
            self.opts,
            None,
            self.identity.as_ref(),
            Some(self.project_opts),
        )
        .await?;
        self.node_name = Some(node_name);
        Ok(self)
    }

    /// Creates and sets a project lookup from a Project Info file
    pub async fn with_project_from_file(
        &mut self,
        file_path: &PathBuf,
    ) -> Result<&mut OrchestratorApiBuilder<'a>> {
        // Read (okta and authority) project parameters from project.json
        let s = tokio::fs::read_to_string(file_path).await?;
        let proj_info: ProjectInfo = serde_json::from_str(&s)?;
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
        let node_name = self.node_name.as_ref().context("No node name")?;
        let config_lookup = self.opts.state.nodes.get(node_name)?.config.lookup;

        let proj = config_lookup
            .get_project(proj_name)
            .context(format!("Unknown project {}", proj_name))?;

        self.project_lookup = Some(proj);
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

    pub fn as_identity(&mut self, identity: Option<String>) -> &mut Self {
        self.identity = identity;
        self
    }

    // TODO oa: will be used within auth flow
    #[allow(dead_code)]
    pub fn use_one_time_code(&mut self, otc: OneTimeCode) -> &mut Self {
        self.one_time_code = Some(otc);
        self
    }

    pub async fn authenticate(&self) -> Result<Credential<'a>> {
        let sc_addr = self
            .secure_channel_to(&OrchestratorEndpoint::Authenticator)
            .await?;

        let authenticator_route = {
            let service = MultiAddr::try_from(
                format!("/service/{}", DefaultAddress::AUTHENTICATOR).as_str(),
            )?;
            let addr = sc_addr.concat(&service)?;
            ockam_api::multiaddr_to_route(&addr).context(format!("Invalid MultiAddr {addr}"))?
        };

        let mut client = Client::new(authenticator_route, self.ctx).await?;

        let credential = match self.one_time_code.clone() {
            None => client.credential().await?,
            Some(token) => client.credential_with(&token).await?,
        };

        Ok(credential.to_owned())
    }

    /// Sends the request and returns  the response
    pub async fn build(&mut self, service_address: &MultiAddr) -> Result<OrchestratorApi<'a>> {
        self.retrieve_project_info().await?;
        // Authenticate with the project authority node
        let _ = self.authenticate().await?;

        //  Establish a secure channel
        let sc_addr = self.secure_channel_to(&self.destination).await?;

        let to = sc_addr.concat(service_address)?;
        info!(
            "creating an rpc client to service: {} over secure channel {}",
            service_address, to
        );

        let node_name = self.node_name.as_ref().context("Node is required")?;
        let rpc = RpcBuilder::new(self.ctx, self.opts, node_name)
            .to(&to)?
            .build();

        Ok(OrchestratorApi { rpc })
    }

    async fn retrieve_project_info(&mut self) -> Result<()> {
        if self.project_lookup.is_some() {
            return Ok(());
        }

        let project_path = match &self.project_opts.project_path {
            Some(p) => p.clone(),
            None => {
                let default_project = self
                    .opts
                    .state
                    .projects
                    .default()
                    .expect("A default project or project parameter is required.");

                default_project.path
            }
        };

        self.with_project_from_file(&project_path).await?;

        Ok(())
    }

    async fn secure_channel_to(&self, endpoint: &OrchestratorEndpoint) -> Result<MultiAddr> {
        let node_name = self.node_name.as_ref().context("Node is required")?;
        let project = self
            .project_lookup
            .as_ref()
            .context("Project is required")?;

        let addr = match endpoint {
            OrchestratorEndpoint::Authenticator => {
                let authority = project
                    .authority
                    .as_ref()
                    .context("Project Authority is required")?;

                create_secure_channel_to_authority(
                    self.ctx,
                    self.opts,
                    node_name,
                    authority,
                    authority.address(),
                    self.identity.clone(),
                )
                .await?
            }
            OrchestratorEndpoint::Project => {
                let project_identity = project
                    .identity_id
                    .as_ref()
                    .context("Project should have identity set")?;
                let project_route = project
                    .node_route
                    .as_ref()
                    .context("Invalid project node route")?;

                create_secure_channel_to_project(
                    self.ctx,
                    self.opts,
                    node_name,
                    None,
                    project_route,
                    &project_identity.to_string(),
                    self.credential_exchange_mode,
                    None, //self.identity.clone(),
                          //FIXME:  passing an identity here is broken.  Credential is retrieved and
                          //associated with the identity,  but that identity object is _not_ the one that
                          //is used latter to establish the security channel (due to clonning, etc).
                          //Not passing identity here works as the embedded node was started with this
                          //identity as the default one anyway.
                )
                .await?
            }
        };

        Ok(addr)
    }
}

pub struct OrchestratorApi<'a> {
    rpc: Rpc<'a>,
}

impl<'a> OrchestratorApi<'a> {
    pub async fn request_with_response<T, R>(&'a mut self, req: RequestBuilder<'_, T>) -> Result<R>
    where
        T: Encode<()>,
        R: Decode<'a, ()>,
    {
        self.request(req).await?;

        self.rpc.is_ok()?;

        info!("Response is OK!");

        self.rpc.parse_response()
    }

    pub async fn request<T>(&mut self, req: RequestBuilder<'_, T>) -> Result<()>
    where
        T: Encode<()>,
    {
        info!("Initializing request to orchestrator");
        self.rpc.request(req).await
    }
}
