use crate::{
    node::util::start_embedded_node,
    util::{api, Rpc},
    CommandGlobalOpts,
};
use anyhow::{anyhow, Result};
use minicbor::{Decode, Encode};
use ockam::Context;
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelResponse, CredentialExchangeMode,
};
use ockam_core::api::{Request, RequestBuilder};
use ockam_identity::IdentityIdentifier;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use tracing::debug;

use super::RpcBuilder;

/// Helps build an Orchestrator API Request
pub struct OrchestratorApiBuilder<'a> {
    ctx: &'a Context,
    opts: &'a CommandGlobalOpts,
    node_name: Option<String>,
    cloud_address: Option<MultiAddr>,
    authorized_identities: Vec<IdentityIdentifier>,
    credential_exchange_mode: CredentialExchangeMode,
}

impl<'a> OrchestratorApiBuilder<'a> {
    pub fn new(ctx: &'a Context, opts: &'a CommandGlobalOpts) -> Self {
        OrchestratorApiBuilder {
            ctx,
            opts,
            node_name: None,
            cloud_address: None,
            authorized_identities: vec![],
            credential_exchange_mode: CredentialExchangeMode::None,
        }
    }

    /// Creates a new embedded node to communicate with the cloud
    pub async fn with_new_embbeded_node(mut self) -> Result<OrchestratorApiBuilder<'a>> {
        let node_name = start_embedded_node(self.ctx, self.opts).await?;
        self.node_name = Some(node_name);
        Ok(self)
    }

    /// Sets the node to use as the client of the request, without starting one.
    pub async fn with_node(mut self, node_name: String) -> Result<OrchestratorApiBuilder<'a>> {
        self.node_name = Some(node_name);
        Ok(self)
    }

    /// Designates the request to the project
    pub async fn to_project(mut self, to: &MultiAddr) -> Result<OrchestratorApiBuilder<'a>> {
        let proto = to
            .first()
            .ok_or(anyhow!("No protocols found within to address"))?;

        if proto.code() != proto::Project::CODE {
            return Err(anyhow!("First protocol within to address is not a project"));
        }

        let proj_name = proto
            .cast::<proto::Project>()
            .ok_or(anyhow!("Unexpected project protocol"))?;

        let config_lookup = self.opts.config.lookup();

        let proj = config_lookup
            .get_project(&proj_name)
            .ok_or(anyhow!("Unknown project {}", proj_name.to_string()))?;

        let identity_id = proj
            .identity_id
            .as_ref()
            .ok_or(anyhow!("Project should have an identity set"))?;

        self.authorized_identities.push(identity_id.clone());
        self.cloud_address = proj.node_route.clone();

        Ok(self)
    }

    /// Designates the request to the project authority
    pub async fn to_project_authority(
        mut self,
        to: &MultiAddr,
    ) -> Result<OrchestratorApiBuilder<'a>> {
        let proto = to
            .first()
            .ok_or(anyhow!("No protocols found within to address"))?;

        if proto.code() != proto::Project::CODE {
            return Err(anyhow!("First protocol within to address is not a project"));
        }

        let proj_name = proto
            .cast::<proto::Project>()
            .ok_or(anyhow!("Unexpected project protocol"))?;

        let config_lookup = self.opts.config.lookup();

        let proj = config_lookup
            .get_project(&proj_name)
            .ok_or(anyhow!("Unknown project {}", proj_name.to_string()))?;

        let auth = proj
            .authority
            .as_ref()
            .ok_or(anyhow!("Project is missing authority"))?;

        let identity_id = auth.identity_id();

        self.authorized_identities.push(identity_id.clone());
        self.cloud_address = Some(auth.address().clone());

        Ok(self)
    }

    /// Designates the request to the project controller
    pub fn to_project_controller(&'a mut self) -> &'a mut Self {
        todo!();
        self
    }

    pub fn with_credential_exchange(mut self, cem: CredentialExchangeMode) -> Self {
        self.credential_exchange_mode = cem;
        self
    }

    /// Sends the request and returns to the response
    pub async fn build(self) -> anyhow::Result<OrchestratorApi<'a>> {
        let node_name = self.node_name.as_ref().ok_or(anyhow!("Node is required"))?;
        let to = self
            .cloud_address
            .as_ref()
            .ok_or(anyhow!("Project destination is required"))?;

        //  Establish a secure channel
        debug!("establishing secure channel to {to}");
        let mut rpc = RpcBuilder::new(self.ctx, self.opts, node_name).build();

        rpc.request(api::create_secure_channel(
            to,
            Some(self.authorized_identities.clone()),
            self.credential_exchange_mode,
        ))
        .await?;

        let res: CreateSecureChannelResponse = rpc.parse_response()?;
        let addr = res.addr()?;

        // Create request to project | project authority | project controller
        let rpc = RpcBuilder::new(self.ctx, self.opts, node_name)
            .to(&addr)?
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
        self.rpc.request(req).await?;

        Ok(self.rpc.parse_response()?)
    }
}
