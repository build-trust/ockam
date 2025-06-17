use crate::cluster::utils::get_cluster;
use clap::Args;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL_ENV;
use ockam_core::env::get_env_ignore_error;
use ockam_node::Context;

#[derive(Clone, Debug, Args, Default)]
pub struct ClusterArg {
    /// The Cluster that hosts the Zone.
    #[arg(long, hide = true)]
    pub cluster: Option<String>,
}

impl ClusterArg {
    pub async fn get_cluster(&self, ctx: &Context, node: &InMemoryNode) -> miette::Result<String> {
        if let Some(cluster) = &self.cluster {
            return Ok(cluster.clone());
        }
        get_cluster(ctx, node).await
    }
}

#[derive(Clone, Debug, Args, Default)]
pub struct HttpApiArgs {
    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long, hide = true)]
    pub use_http_api: bool,

    /// The API endpoint of the Ockam AI Platform.
    /// Can be set using the `AI_API_BASE_URL` environment variable.
    /// Defaults to `http://localhost:30080`.
    #[arg(long, hide = true)]
    pub api_endpoint: Option<String>,
}

impl HttpApiArgs {
    pub fn from_api_endpoint(api_endpoint: String) -> Self {
        Self {
            use_http_api: true,
            api_endpoint: Some(api_endpoint),
        }
    }

    pub fn use_http_api(&self) -> bool {
        if let Some(api_endpoint) = &self.api_endpoint {
            std::env::set_var(AI_API_BASE_URL_ENV, api_endpoint);
        }
        self.use_http_api
            || self.api_endpoint.is_some()
            || get_env_ignore_error::<String>(AI_API_BASE_URL_ENV).is_some()
    }
}
