use crate::cluster::utils::get_api_client;
use crate::cluster::zone_config::ZoneConfig;
use crate::node_command::InMemoryNodeCommand;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::orchestrator::ai_platform::node_service_client::AI_API_BASE_URL_ENV;
use ockam_api::orchestrator::ai_platform::responses::EcrCredentials;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use std::process::Stdio;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Deploy an Ockam AI Agent into a Zone
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// The name of the Zone to deploy in the Ockam AI Platform
    #[arg(long)]
    pub zone_name: String,

    /// The path to the Zone configuration file, in yaml or json format.
    ///
    /// If not set, the `./zone.json` file from the current directory will be used.
    #[arg(long, visible_alias = "config")]
    pub zone_config: Option<String>,

    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,

    // === Specific args for the HTTP API endpoint
    /// Force the command to use the HTTP API.
    /// By default, the command will use the Orchestrator API.
    #[arg(long)]
    pub use_http_api: bool,

    /// The Cluster that will be used to set up the Zone.
    /// If not set, it will be retrieved from the enrolled user data.
    #[arg(long)]
    pub cluster: Option<String>,

    /// The API endpoint of the Ockam AI Platform.
    /// Defaults to `http://localhost:30080`.
    #[arg(long)]
    pub api_endpoint: Option<String>,
}

#[derive(Clone)]
struct CreateNodeCommand {
    opts: CommandGlobalOpts,
    command: CreateCommand,
}

#[async_trait]
impl InMemoryNodeCommand for CreateNodeCommand {
    async fn init(&self) -> miette::Result<()> {
        if let Some(api_endpoint) = &self.command.api_endpoint {
            std::env::set_var(AI_API_BASE_URL_ENV, api_endpoint);
        }
        Ok(())
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let api_client = get_api_client(&node, self.command.use_http_api).await?;
        let cluster = api_client.get_cluster(ctx).await?;
        let zone_config = self
            .command
            .process_images(ctx, &self.opts, &*api_client, &cluster)
            .await?;
        self.command
            .deploy_zone(ctx, &self.opts, &*api_client, &cluster, zone_config)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "cluster create";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = CreateNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

impl CreateCommand {
    async fn process_images(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
    ) -> Result<ZoneConfig> {
        let zone_config_path = self.zone_config.as_deref().unwrap_or("./zone.json");
        let mut zone_config = ZoneConfig::from_file(zone_config_path)?;

        let config_images = zone_config.get_local_images_names();
        if config_images.is_empty() {
            opts.terminal
                .write_line(fmt_log!("No local images found in zone config"))?;
            return Ok(zone_config);
        }

        for image_name in config_images {
            let ecr_creds = self
                .provision_ecr(ctx, opts, api_client, cluster, &image_name)
                .await?;
            self.push_image(opts, &image_name, &ecr_creds).await?;
            // TODO: remove the role
            // node.delete_ecr_role(cluster, &self.zone_name, &image_name)
            //     .await?;
            zone_config.replace_image_name(&image_name, &ecr_creds.repository_uri)?;
        }
        Ok(zone_config)
    }

    async fn provision_ecr(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        image_name: &str,
    ) -> Result<EcrCredentials> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Provisioning ECR in cluster {} for image {}...",
                color_primary(cluster),
                color_primary(image_name),
            ));
        }
        let ecr_creds = api_client
            .provision_ecr(ctx, cluster, image_name, Some(self.use_public_ecr))
            .await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_log!(
            "Provisioned ECR in cluster {} for image {} at {}",
            color_primary(cluster),
            color_primary(image_name),
            color_primary(&ecr_creds.repository_uri)
        ))?;

        Ok(ecr_creds)
    }

    async fn push_image(
        &self,
        opts: &CommandGlobalOpts,
        local_image_name: &str,
        ecr_credentials: &EcrCredentials,
    ) -> Result<()> {
        let spinner = opts.terminal.spinner();

        // Login
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message("Logging docker into ECR...");
        }
        let mut child = tokio::process::Command::new("docker")
            .arg("login")
            .arg("-u")
            .arg("AWS")
            .arg("--password-stdin")
            .arg(&ecr_credentials.repository_uri)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .into_diagnostic()?;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(ecr_credentials.auth_token.as_bytes())
                .await
                .into_diagnostic()?;
        }
        let output = child.wait_with_output().await.into_diagnostic()?;
        if !output.status.success() {
            return Err(miette::Error::msg(format!(
                "Failed to login to ECR: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Tag image
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Tagging image {} with ECR's URI...",
                color_primary(local_image_name),
            ));
        }
        let output = tokio::process::Command::new("docker")
            .arg("tag")
            .arg(local_image_name)
            .arg(&ecr_credentials.repository_uri)
            .output()
            .await
            .into_diagnostic()?;
        if !output.status.success() {
            return Err(miette::Error::msg(format!(
                "Failed to tag image {}: {}",
                local_image_name,
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Push image
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Pushing image {} into ECR...",
                color_primary(local_image_name),
            ));
        }

        let output = tokio::process::Command::new("docker")
            .arg("push")
            .arg(&ecr_credentials.repository_uri)
            .output()
            .await
            .into_diagnostic()?;
        if !output.status.success() {
            return Err(miette::Error::msg(format!(
                "Failed to push image {}: {}",
                local_image_name,
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Pushed image {} into ECR\n",
            color_primary(local_image_name),
        ))?;

        Ok(())
    }

    async fn deploy_zone(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        zone_config: ZoneConfig,
    ) -> Result<()> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Deploying zone {} in cluster {}...",
                color_primary(&self.zone_name),
                color_primary(cluster),
            ));
        }

        // TODO: do we want to recreate the zone every time?
        //  Is there another way of reapplying the configuration for an existing zone?
        let _ = api_client.delete_zone(ctx, cluster, &self.zone_name).await;
        api_client
            .create_zone(ctx, cluster, &self.zone_name)
            .await?;
        // TODO: how do we pass the secrets to the command? maybe as a json/yaml file?
        // api_client.create_secret(ctx, cluster, &self.zone_name, secret_name, secret_fields).await?;

        let zone_config_json = serde_json::to_value(&zone_config).into_diagnostic()?;
        api_client
            .deploy_zone(ctx, cluster, &self.zone_name, &zone_config_json)
            .await?;

        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Deployed zone {} in cluster {}",
            color_primary(&self.zone_name),
            color_primary(cluster)
        ))?;

        Ok(())
    }
}
