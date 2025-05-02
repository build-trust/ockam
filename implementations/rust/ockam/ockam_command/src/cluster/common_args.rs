use crate::cluster::utils::get_cluster;
use crate::cluster::zone_config::ZoneConfig;
use clap::Args;
use miette::{miette, IntoDiagnostic};
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL_ENV;
use ockam_core::env::get_env_ignore_error;
use ockam_node::Context;

#[derive(Clone, Debug, Args, Default)]
pub struct ClusterArg {
    /// The Cluster that hosts the Zone.
    #[arg(long)]
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
pub struct ZoneConfigArg {
    /// The path to the Zone configuration file, in yaml or json format.
    /// If not set, the `./ockam.yaml` file from the current directory will be used.
    #[arg(long, visible_alias = "config")]
    pub zone_config: Option<String>,
}

#[derive(Clone, Debug, Args, Default)]
#[group(multiple = false)]
pub struct ZoneNameOrConfigArg {
    /// The name of the Zone
    #[arg(long)]
    pub zone_name: Option<String>,

    #[command(flatten)]
    pub zone_config: ZoneConfigArg,
}

impl From<ZoneConfigArg> for ZoneNameOrConfigArg {
    fn from(zone_config: ZoneConfigArg) -> Self {
        Self {
            zone_name: None,
            zone_config,
        }
    }
}

impl ZoneNameOrConfigArg {
    pub fn from_zone_name(zone_name: String) -> Self {
        Self {
            zone_name: Some(zone_name),
            zone_config: ZoneConfigArg::default(),
        }
    }

    pub fn zone_name(&self) -> crate::Result<String> {
        if let Some(zone_name) = &self.zone_name {
            return Ok(zone_name.clone());
        }
        let zone_config_path = match &self.zone_config.zone_config {
            Some(path) => path,
            None => {
                if std::path::Path::new("./ockam.yaml")
                    .try_exists()
                    .into_diagnostic()?
                {
                    "./ockam.yaml"
                } else if std::path::Path::new("./ockam.yml")
                    .try_exists()
                    .into_diagnostic()?
                {
                    "./ockam.yml"
                } else {
                    return Err(miette!("Zone config file not found. Please provide a zone name or a zone config file."));
                }
            }
        };
        let zone_config = ZoneConfig::from_file(zone_config_path)?;
        Ok(zone_config.name)
    }
}

#[derive(Clone, Debug, Args, Default)]
pub struct HttpApiArgs {
    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long)]
    pub use_http_api: bool,

    /// The API endpoint of the Ockam AI Platform.
    /// Can be set using the `AI_API_BASE_URL` environment variable.
    /// Defaults to `http://localhost:30080`.
    #[arg(long)]
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

#[derive(Clone, Debug, Args, Default)]
pub struct SecretsConfigArg {
    /// The path to the secrets file, in yaml or json format.
    /// If not set, the `./secrets.yaml` file from the current directory will be used.
    /// If no file is found, the command will just list the existing secrets.
    #[arg(long, visible_alias = "secrets")]
    pub secrets_config: Option<String>,
}
