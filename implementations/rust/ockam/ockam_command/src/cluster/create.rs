use crate::cluster::common_args::{HttpApiArgs, SecretsConfigArg, ZoneConfigArg};
use crate::cluster::secret::SecretCommand;
use crate::cluster::utils::{get_api_client, get_cluster};
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
use ockam_api::orchestrator::ai_platform::responses::EcrCredentials;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use std::process::Stdio;
use std::sync::Arc;
use tracing::info;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Deploy an Ockam AI Agent into a Zone
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[command(flatten)]
    pub zone_config: ZoneConfigArg,

    #[command(flatten)]
    pub secrets_config: SecretsConfigArg,

    /// Whether to use a public AWS ECR
    #[arg(long)]
    pub use_public_ecr: bool,

    /// Whether to use the Docker cache when building the image.
    /// It can be set using the `OCKAM_IGNORE_DOCKER_CACHE` environment variable.
    #[arg(long, env = "OCKAM_IGNORE_DOCKER_CACHE")]
    pub ignore_docker_cache: bool,

    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

#[derive(Clone)]
struct CreateNodeCommand {
    opts: CommandGlobalOpts,
    command: CreateCommand,
}

#[async_trait]
impl InMemoryNodeCommand<ZoneConfig> for CreateNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<ZoneConfig> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = get_cluster(ctx, &node).await?;
        let parsed_zone_config = self.command.parse_zone_config()?;

        // Delete the zone is relatively expensive operation, so we do it in parallel with the image processing
        let zone_config_future = self.command.process_images(
            ctx,
            parsed_zone_config.clone(),
            &self.opts,
            &*api_client,
            &cluster,
        );
        let delete_zone_future = api_client.delete_zone(ctx, &cluster, &parsed_zone_config.name);

        let (zone_config, _) = tokio::join!(zone_config_future, delete_zone_future);
        // We don't check the result of the delete zone operation, it can fail if the zone doesn't exist
        let zone_config = zone_config?;

        self.command
            .deploy_zone(ctx, &self.opts, &*api_client, &cluster, &zone_config)
            .await?;
        Ok(zone_config)
    }
}

#[async_trait]
impl Command<ZoneConfig> for CreateCommand {
    const NAME: &'static str = "cluster create";

    async fn run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> Result<ZoneConfig> {
        let command = CreateNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        let zone_config = command.execute(ctx, opts.state.clone()).await?;
        Ok(zone_config)
    }
}

impl CreateCommand {
    fn parse_zone_config(&self) -> Result<ZoneConfig> {
        let zone_config_path = self
            .zone_config
            .zone_config
            .as_deref()
            .unwrap_or("./ockam.yaml");
        ZoneConfig::from_file(zone_config_path)
    }

    async fn process_images(
        &self,
        ctx: &Context,
        mut zone_config: ZoneConfig,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
    ) -> Result<ZoneConfig> {
        let config_images = zone_config.get_local_images_names();
        if config_images.is_empty() {
            opts.terminal
                .write_line(fmt_log!("No local images found in zone config"))?;
            return Ok(zone_config);
        }

        // TODO: these likely should be done in parallel
        for image_name in config_images {
            let ecr_creds = self
                .provision_ecr(ctx, opts, api_client, cluster, &image_name)
                .await?;
            self.docker_login(opts, &ecr_creds).await?;
            let repository_url_tag = self
                .build_and_push_local_image(opts, &image_name, &ecr_creds.repository_uri)
                .await?;
            // TODO: remove the role
            // node.delete_ecr_role(cluster, &self.zone_name, &image_name)
            //     .await?;
            zone_config.replace_image_name(&image_name, &repository_url_tag)?;
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
                "Creating a repository for the {} image...",
                color_primary(image_name),
            ));
        }
        let ecr_creds = api_client
            .provision_ecr(ctx, cluster, image_name, Some(self.use_public_ecr))
            .await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Created a repository for the {} image\n",
            color_primary(image_name),
        ))?;
        info!(
            "Created a repository for the image {} in cluster {} at {}",
            image_name, cluster, ecr_creds.repository_uri
        );

        Ok(ecr_creds)
    }

    async fn docker_login(
        &self,
        opts: &CommandGlobalOpts,
        ecr_credentials: &EcrCredentials,
    ) -> Result<()> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message("Giving docker access to the repository...");
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
        if let Some(spinner) = spinner.as_ref() {
            spinner.finish_and_clear();
        }
        if !output.status.success() {
            return Err(miette::Error::msg(format!(
                "Failed to login into repository: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    async fn build_and_push_local_image(
        &self,
        opts: &CommandGlobalOpts,
        image_name: &str,
        repository_uri: &str,
    ) -> Result<String> {
        // Given an image name, try to build the `Dockerfile` image at "./images/{image_name}/Dockerfile"
        let dockerfile_dir = format!("./images/{}", image_name);
        if !std::path::Path::new(&dockerfile_dir)
            .try_exists()
            .into_diagnostic()?
        {
            return Ok(String::new());
        }

        let version = std::time::SystemTime::now();
        let repository_uri_tag = format!(
            "{}:{}",
            repository_uri,
            version
                .duration_since(std::time::UNIX_EPOCH)
                .into_diagnostic()?
                .as_secs()
        );

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Building image {} with tag {}...",
                color_primary(image_name),
                color_primary(&repository_uri_tag)
            ));
        }

        let build_attempts = [
            // With buildkit enabled
            (
                vec![
                    "build",
                    "--load",
                    "--platform",
                    "linux/amd64",
                    "-t",
                    &repository_uri_tag,
                    ".",
                ],
                vec![("DOCKER_BUILDKIT", "1")],
            ),
            // With buildkit disabled
            (
                vec![
                    "build",
                    "--platform",
                    "linux/amd64",
                    "-t",
                    &repository_uri_tag,
                    ".",
                ],
                vec![("DOCKER_BUILDKIT", "0")],
            ),
        ];

        let mut last_error = None;

        // Try each command until one succeeds
        for (cmd_args, env_vars) in &build_attempts {
            let mut command = tokio::process::Command::new("docker");
            for arg in cmd_args {
                command.arg(arg);
            }
            if self.ignore_docker_cache {
                command.arg("--no-cache");
            }
            for (env_name, env_value) in env_vars {
                command.env(env_name, env_value);
            }

            info!(
                "Attempting docker build for {} with command: docker {} and env: {:?}",
                image_name,
                cmd_args.join(" "),
                env_vars
            );

            let output = command
                .current_dir(&dockerfile_dir)
                .output()
                .await
                .into_diagnostic()?;

            if output.status.success() {
                // Command succeeded
                if let Some(spinner) = spinner.as_ref() {
                    spinner.finish_and_clear();
                }
                opts.terminal.write_line(fmt_ok!(
                    "Built local image {} with tag {}",
                    color_primary(image_name),
                    color_primary(&repository_uri_tag)
                ))?;

                // Push image
                let push_spinner = opts.terminal.spinner();
                if let Some(spinner) = push_spinner.as_ref() {
                    spinner.set_message(format!("Pushing image {}...", color_primary(image_name)));
                }
                let push_output = tokio::process::Command::new("docker")
                    .arg("push")
                    .arg(&repository_uri_tag)
                    .output()
                    .await
                    .into_diagnostic()?;

                if !push_output.status.success() {
                    return Err(miette::Error::msg(format!(
                        "Failed to push image {}: {}",
                        image_name,
                        String::from_utf8_lossy(&push_output.stderr)
                    )));
                }

                if let Some(spinner) = push_spinner {
                    spinner.finish_and_clear();
                }

                opts.terminal
                    .write_line(fmt_ok!("Pushed image {}\n", color_primary(image_name)))?;

                return Ok(repository_uri_tag);
            } else {
                // Store the error for reporting if all commands fail
                last_error = Some(format!(
                    "Docker build failed with command 'docker {}' and env vars {:?}: {}",
                    cmd_args.join(" "),
                    env_vars,
                    String::from_utf8_lossy(&output.stderr)
                ));

                // Log the failed attempt and continue to the next command
                info!(
                    "Docker build attempt failed for {}: {}",
                    image_name,
                    last_error.as_ref().unwrap()
                );
            }
        }

        // If we get here, all commands failed
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }

        Err(miette::Error::msg(format!(
            "Failed to build image {}: {}",
            image_name,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    async fn deploy_zone(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        zone_config: &ZoneConfig,
    ) -> Result<()> {
        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Deploying zone {}...",
                color_primary(&zone_config.name),
            ));
        }

        api_client
            .create_zone(ctx, cluster, &zone_config.name)
            .await?;
        {
            let mut opts = opts.clone();
            opts.terminal = opts.terminal.disable();
            SecretCommand {
                secrets_config: self.secrets_config.clone(),
                zone: self.zone_config.clone().into(),
                http_api: self.http_api.clone(),
            }
            .push_secrets(ctx, &opts, api_client, cluster, &zone_config.name)
            .await?;
        }

        let zone_config_json = serde_json::to_value(zone_config).into_diagnostic()?;
        api_client
            .deploy_zone(ctx, cluster, &zone_config.name, &zone_config_json)
            .await?;

        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Deployed zone {} in cluster {}\n",
            color_primary(&zone_config.name),
            color_primary(cluster)
        ))?;
        info!("Deployed zone {} in cluster {}", zone_config.name, cluster);

        Ok(())
    }
}
