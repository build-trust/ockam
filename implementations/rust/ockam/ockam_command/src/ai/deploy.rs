use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::orchestrator::ai_platform::models::EcrCredentials;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use std::process::Stdio;

use crate::run::parser::resource::zone_config::ZoneConfig;
use crate::{docs, Command, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/deploy/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/deploy/after_long_help.txt");

/// Deploy an Ockam AI Agent into a Zone
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeployCommand {
    /// The name of the zone to deploy in the Ockam AI Platform
    #[arg(long)]
    pub zone_name: String,

    /// The path to the zone configuration file, in yaml or json format.
    ///
    /// If not set, the `./zone.json` file from the current directory will be used.
    #[arg(long)]
    pub zone_config: Option<String>,

    /// The region of the AWS ECR to use.
    ///
    /// If using a public AWS ECR, the region will be set to "us-east-1", which is the
    /// only region that supports public ECRs.
    #[arg(long)]
    pub ecr_region: Option<String>,

    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,
}

#[async_trait]
impl Command for DeployCommand {
    const NAME: &'static str = "ai deploy";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let node = InMemoryNode::start(ctx, opts.state.clone()).await?;
        let customer_name = self.get_customer_name(&opts).await?;

        let zone_config = self.process_images(&node, &opts, &customer_name).await?;
        self.deploy_zone(&node, &opts, &customer_name, zone_config)
            .await?;
        Ok(())
    }
}

impl DeployCommand {
    async fn get_customer_name(&self, opts: &CommandGlobalOpts) -> Result<String> {
        let customer = opts
            .state
            .get_default_user()
            .await?
            .email
            .domain()?
            .replace('.', "-");
        opts.terminal.write_line(fmt_log!(
            "Retrieved customer {} from enrolled user data\n",
            color_primary(&customer),
        ))?;
        Ok(customer)
    }

    async fn process_images(
        &self,
        node: &InMemoryNode,
        opts: &CommandGlobalOpts,
        customer: &str,
    ) -> Result<ZoneConfig> {
        let zone_config_path = self.zone_config.as_deref().unwrap_or("./zone.json");
        let mut zone_config = ZoneConfig::from_file(zone_config_path)?;

        for image_name in zone_config.get_local_images_names() {
            let ecr_creds = self
                .provision_ecr(node, opts, customer, &image_name)
                .await?;
            self.push_image(opts, &image_name, &ecr_creds).await?;
            // TODO: remove the role
            // node.delete_ecr_role(customer, &self.zone_name, &image_name)
            //     .await?;
            zone_config.replace_image_name(&image_name, &ecr_creds.repository_uri)?;
        }
        Ok(zone_config)
    }

    async fn provision_ecr(
        &self,
        node: &InMemoryNode,
        opts: &CommandGlobalOpts,
        customer: &str,
        image_name: &str,
    ) -> Result<EcrCredentials> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Provisioning ECR for image {}...",
                color_primary(image_name),
            ));
        }
        let region = if self.use_public_ecr {
            Some("us-east-1")
        } else {
            self.ecr_region.as_deref()
        };
        let ecr_creds = node
            .provision_ecr(customer, image_name, region, Some(self.use_public_ecr))
            .await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_log!(
            "Provisioned ECR for image {} at {}",
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
        node: &InMemoryNode,
        opts: &CommandGlobalOpts,
        customer: &str,
        zone_config: ZoneConfig,
    ) -> Result<()> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Deploying zone {}...",
                color_primary(&self.zone_name),
            ));
        }

        // TODO: do we want to recreate the zone every time?
        //  Is there another way of reapplying the configuration for an existing zone?
        let _ = node.delete_zone(customer, &self.zone_name).await;
        node.create_zone(customer, &self.zone_name).await?;
        // TODO: how do we pass the secrets to the command? maybe as a json/yaml file?
        // node.create_secret(customer, &self.zone_name, secret_name, secret_fields).await?;

        let zone_config_json = serde_json::to_value(&zone_config).into_diagnostic()?;
        node.deploy_zone(customer, &self.zone_name, &zone_config_json)
            .await?;

        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal
            .write_line(fmt_ok!("Deployed zone {}", color_primary(&self.zone_name),))?;

        Ok(())
    }
}
