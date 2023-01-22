use std::path::PathBuf;

use crate::{
    credential,
    node::util::{delete_embedded_node, delete_node, start_embedded_node},
    project::{
        util::{create_secure_channel_to_authority, create_secure_channel_to_project},
        ProjectInfo,
    },
    util::{api, Rpc},
    CommandGlobalOpts,
};
use anyhow::{anyhow, Result};
use minicbor::{Decode, Encode};
use ockam::Context;
use ockam_api::{
    authenticator::direct::{types::OneTimeCode, Client},
    cloud::project::Project,
    config::lookup::ProjectLookup,
    nodes::models::secure_channel::{
        CreateSecureChannelRequest, CreateSecureChannelResponse, CredentialExchangeMode,
    },
    DefaultAddress,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_identity::{credential::Credential, IdentityIdentifier};
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use tracing::{debug, info};

use super::RpcBuilder;

pub enum OrchestratorEndpoint {
    Authenticator,
    Project,
}
/// Helps build an Orchestrator API Request
pub struct OrchestratorApiBuilder<'a> {
    ctx: &'a Context,
    opts: &'a CommandGlobalOpts,
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
    pub fn new(ctx: &'a Context, opts: &'a CommandGlobalOpts) -> Self {
        OrchestratorApiBuilder {
            ctx,
            opts,
            node_name: None,
            destination: OrchestratorEndpoint::Project,
            identity: None,
            credential_exchange_mode: CredentialExchangeMode::Oneway,
            project_lookup: None,
            one_time_code: None,
        }
    }

    /// Creates a new embedded node to communicate with the cloud
    pub async fn with_new_embbeded_node(&mut self) -> Result<&mut OrchestratorApiBuilder<'a>> {
        let node_name = start_embedded_node(self.ctx, self.opts).await?;
        self.node_name = Some(node_name);
        Ok(self)
    }

    /// Sets the node to use as the client of the request, without starting one.
    pub fn with_node(&mut self, node_name: String) -> &mut Self {
        self.node_name = Some(node_name);
        self
    }

    pub async fn with_project_from_file(
        &mut self,
        file_path: &PathBuf,
    ) -> Result<&mut OrchestratorApiBuilder<'a>> {
        // Read (okta and authority) project parameters from project.json
        let s = tokio::fs::read_to_string(file_path).await?;
        let p: ProjectInfo = serde_json::from_str(&s)?;

        let project = Project::from(&p);
        let project_lookup = ProjectLookup::from_project(&project).await?;

        self.project_lookup = Some(project_lookup);
        Ok(self)
    }

    pub async fn with_project_from_lookup(
        &mut self,
        proj_name: &str,
    ) -> Result<&mut OrchestratorApiBuilder<'a>> {
        let config_lookup = self.opts.config.lookup();

        let proj = config_lookup
            .get_project(&proj_name)
            .ok_or(anyhow!("Unknown project {}", proj_name.to_string()))?;

        self.project_lookup = Some(proj.clone());
        Ok(self)
    }

    pub fn to_orchestor_endpoint(&mut self, destination: OrchestratorEndpoint) -> &mut Self {
        self.destination = destination;
        self
    }

    pub fn with_credential_exchange(&mut self, cem: CredentialExchangeMode) -> &mut Self {
        self.credential_exchange_mode = cem;
        self
    }

    pub fn as_identity(&mut self, identity: Option<String>) -> &mut Self {
        self.identity = identity;
        self
    }

    pub fn use_one_time_code(&mut self, otc: OneTimeCode) -> &mut Self {
        self.one_time_code = Some(otc);
        self
    }

    pub async fn authenticate(&self) -> Result<Credential<'a>> {
        let node_name = self.node_name.as_ref().ok_or(anyhow!("Node is required"))?;
        let project = self
            .project_lookup
            .as_ref()
            .ok_or(anyhow!("Project is required"))?;

        let authority = project
            .authority
            .as_ref()
            .ok_or(anyhow!("Project Authority is required"))?;

        let sc_addr = create_secure_channel_to_authority(
            self.ctx,
            self.opts,
            node_name,
            authority,
            authority.address(),
            self.identity.clone(),
        )
        .await?;

        let authenticator_route = {
            let service = MultiAddr::try_from(
                format!("/service/{}", DefaultAddress::AUTHENTICATOR).as_str(),
            )?;
            let addr = sc_addr.concat(&service)?;
            ockam_api::multiaddr_to_route(&addr).ok_or(anyhow!("Invalid MultiAddr {}", addr))?
        };

        let mut client = Client::new(authenticator_route, self.ctx).await?;

        let credential = match self.one_time_code.clone() {
            None => client.credential().await?,
            Some(token) => client.credential_with(&token).await?,
        };

        Ok(credential.to_owned())
    }

    /// Sends the request and returns  the response
    pub async fn build(&self, service_address: &MultiAddr) -> anyhow::Result<OrchestratorApi<'a>> {
        let project = self
            .project_lookup
            .as_ref()
            .ok_or(anyhow!("Project is required"))?;
        let project_identity = project
            .identity_id
            .as_ref()
            .ok_or(anyhow!("Project should have identity set"))?;
        let project_route = project
            .node_route
            .as_ref()
            .ok_or(anyhow!("Invalid project node route"))?;

        let node_name = self.node_name.as_ref().ok_or(anyhow!("Node is required"))?;

        // Authenticate with the project authority node
        let _ = self.authenticate().await?;

        //  Establish a secure channel
        info!("establishing secure channel to {project_route}");
        let sc_addr = create_secure_channel_to_project(
            self.ctx,
            self.opts,
            node_name,
            None,
            project_route,
            &project_identity.to_string(),
            self.credential_exchange_mode,
            self.identity.clone(),
        )
        .await?;

        let to = sc_addr.concat(service_address)?;
        info!(
            "creating an rpc client to service: {} over secure channel {}",
            service_address, to
        );

        let rpc = RpcBuilder::new(self.ctx, self.opts, node_name)
            .to(&to)?
            .build();

        Ok(OrchestratorApi { rpc })
    }
}

pub struct OrchestratorApi<'a> {
    rpc: Rpc<'a>,
}
impl<'a> OrchestratorApi<'a> {
    pub async fn request<T, R>(&'a mut self, req: RequestBuilder<'_, T>) -> anyhow::Result<R>
    where
        T: Encode<()>,
        R: Decode<'a, ()>,
    {
        info!("Initializing request to orchestrator");
        self.rpc.request(req).await?;

        info!("request sent!");
        self.rpc.is_ok()?;

        info!("Response is OK!");

        Ok(self.rpc.parse_response()?)
    }
}
