use crate::cluster::common_args::HttpApiArgs;
use crate::cluster::utils::{get_api_client, get_cluster};
use crate::node_command::InMemoryNodeCommand;
use crate::zone::common_args::{SecretsConfigArg, ZoneConfigArg};
use crate::zone::secret::SecretCommand;
use crate::zone::zone_config::ZoneConfig;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use indicatif::ProgressBar;
use miette::IntoDiagnostic;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::orchestrator::ai_platform::responses::EcrCredential;
use ockam_api::{fmt_log, fmt_ok};
use ockam_node::Context;
use std::process::Stdio;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, info};

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
    pub no_docker_cache: bool,

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

        self.opts.terminal.write_line(fmt_log!(
            "Deploying zone {} in cluster {}...\n",
            color_primary(&parsed_zone_config.name),
            color_primary(&cluster),
        ))?;

        // Delete the zone is a relatively slow operation, so we do it in parallel with the image processing
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
    const NAME: &'static str = "zone create";

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
        let images_names = zone_config.get_local_images_names();
        if images_names.is_empty() {
            opts.terminal
                .write_line(fmt_log!("No local images found in zone config"))?;
            return Ok(zone_config);
        }

        // Provision ECR credentials
        let ecr_cred = self
            .provision_ecr(ctx, opts, api_client, cluster, images_names.clone())
            .await?;

        // Login and build images in parallel.
        // When the images are cached, the login operation will take longer than the build operation,
        // so we share a spinner to wait for both operations.
        let spinner = opts.terminal.spinner();
        let login_task = {
            let _self = self.clone();
            let ecr_cred = ecr_cred.clone();
            tokio::task::spawn(async move {
                // The login operations can't be parallelized, but we can run it in a separate task
                _self.docker_login(&ecr_cred).await
            })
        };
        let build_task = self.build_docker_images(spinner.as_ref(), &ecr_cred);
        let tasks_res = tokio::join!(login_task, build_task);
        tasks_res.0.into_diagnostic()??;
        let (images_names_formatter, images_data_res) = tasks_res.1?;
        let mut images_data = vec![];
        for image_data_res in images_data_res {
            let image_data = image_data_res?;
            images_data.push(image_data);
        }
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal
            .write_line(fmt_ok!("Built {}", &images_names_formatter.format()))?;

        // Update zone config
        for image_data in &images_data {
            // TODO: remove the role
            // node.delete_ecr_role(cluster, &self.zone_name, &image_name)
            //     .await?;
            zone_config
                .replace_image_name(&image_data.image_name, &image_data.repository_uri_tag)?;
        }

        // Push the images
        self.push_local_images(opts, &images_data).await?;

        opts.terminal.write_line("")?;

        Ok(zone_config)
    }

    async fn provision_ecr(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        image_names: Vec<String>,
    ) -> Result<EcrCredential> {
        let images_names_formatter = ImagesNamesFormatter::new(image_names.clone());

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!(
                "Creating repositories for the {}...",
                images_names_formatter.format()
            ));
        }
        let ecr_creds = api_client
            .provision_ecr(ctx, cluster, image_names.clone(), Some(self.use_public_ecr))
            .await?;
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal.write_line(fmt_ok!(
            "Created a repository for the {}",
            images_names_formatter.format()
        ))?;
        for (image_name, uri) in ecr_creds.images.iter() {
            info!(
                "Created a repository for the image {} in cluster {} at {}",
                color_primary(image_name),
                cluster,
                uri
            );
        }

        Ok(ecr_creds)
    }

    async fn build_docker_images(
        &self,
        spinner: Option<&ProgressBar>,
        ecr_cred: &EcrCredential,
    ) -> Result<(ImagesNamesFormatter, Vec<Result<ImageData>>)> {
        let mut images_names_formatter =
            ImagesNamesFormatter::new(ecr_cred.images.keys().cloned().collect());
        images_names_formatter.dim();

        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!("Building {}...", images_names_formatter.format()));
        }

        let mut set: JoinSet<Result<ImageData>> = JoinSet::new();
        for (image_name, repository_uri) in ecr_cred.images.iter() {
            let _self = self.clone();
            let image_name = image_name.clone();
            let repository_uri = repository_uri.clone();
            set.spawn(async move {
                let repository_uri_tag = _self
                    .build_local_image(&image_name, &repository_uri)
                    .await?;
                Ok(ImageData {
                    image_name,
                    repository_uri_tag,
                })
            });
        }
        let mut build_res = vec![];
        while let Some(res) = set.join_next().await {
            let image_data = res.into_diagnostic()?;
            if let Ok(image_data) = &image_data {
                images_names_formatter.update_name(
                    &image_data.image_name,
                    color_primary(&image_data.image_name).to_string(),
                );
                if let Some(spinner) = spinner.as_ref() {
                    spinner.set_message(format!("Building {}...", images_names_formatter.format()));
                }
            }
            build_res.push(image_data);
        }
        Ok((images_names_formatter, build_res))
    }

    async fn build_local_image(&self, image_name: &str, repository_uri: &str) -> Result<String> {
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

        let build_attempts = [
            // With buildkit enabled
            (
                vec![
                    "build",
                    "--load",
                    "--pull",
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
                    "--pull",
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
            if self.no_docker_cache {
                command.arg("--no-cache");
            }
            for (env_name, env_value) in env_vars {
                command.env(env_name, env_value);
            }

            info!(
                "attempting docker build for {} with command: '{}' and env: {:?}",
                image_name,
                color_primary(format!("docker {}", cmd_args.join(" "))),
                env_vars
            );

            let output = command
                .current_dir(&dockerfile_dir)
                .output()
                .await
                .into_diagnostic()?;

            if output.status.success() {
                info!(
                    "built local image {} with tag {}",
                    color_primary(image_name),
                    color_primary(&repository_uri_tag)
                );
                return Ok(repository_uri_tag);
            } else {
                // Store the error for reporting if all commands fail
                last_error = Some(format!(
                    "Docker build failed with command '{}' and env vars {:?}: {}",
                    color_primary(format!("docker {}", cmd_args.join(" "))),
                    env_vars,
                    String::from_utf8_lossy(&output.stderr)
                ));

                // Log the failed attempt and continue to the next command
                info!(
                    "docker build attempt failed for {}: {}",
                    image_name,
                    last_error.as_ref().unwrap()
                );
            }
        }

        Err(miette::Error::msg(format!(
            "Failed to build image {}: {}",
            image_name,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }

    async fn docker_login(&self, ecr_credentials: &EcrCredential) -> Result<()> {
        debug!(
            "logging into ECR with credentials: {}",
            ecr_credentials.auth_token
        );
        for (_image_name, uri) in ecr_credentials.images.iter() {
            debug!("logging into ECR with uri: {}", color_primary(uri));
            let mut child = tokio::process::Command::new("docker")
                .arg("login")
                .arg("-u")
                .arg("AWS")
                .arg("--password-stdin")
                .arg(uri)
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
                    "Failed to login into repository: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }
        info!("logged into ECR successfully");
        Ok(())
    }

    async fn push_local_images(
        &self,
        opts: &CommandGlobalOpts,
        images_data: &[ImageData],
    ) -> Result<()> {
        let mut images_names_formatter =
            ImagesNamesFormatter::new(images_data.iter().map(|x| x.image_name.clone()).collect());
        images_names_formatter.dim();

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = spinner.as_ref() {
            spinner.set_message(format!("Pushing {}...", images_names_formatter.format()));
        }

        let mut set: JoinSet<Result<String>> = JoinSet::new();
        for image_data in images_data.iter() {
            let _self = self.clone();
            let image_data = image_data.clone();
            set.spawn(async move {
                _self
                    .push_local_image(&image_data.image_name, &image_data.repository_uri_tag)
                    .await?;
                _self
                    .delete_local_image(&image_data.image_name, &image_data.repository_uri_tag)
                    .await?;
                Ok(image_data.image_name)
            });
        }
        while let Some(res) = set.join_next().await {
            let image_name = res.into_diagnostic()??;
            images_names_formatter.update_name(&image_name, color_primary(&image_name).to_string());
            if let Some(spinner) = spinner.as_ref() {
                spinner.set_message(format!("Pushing {}...", images_names_formatter.format()));
            }
        }
        if let Some(spinner) = spinner {
            spinner.finish_and_clear();
        }
        opts.terminal
            .write_line(fmt_ok!("Pushed {}", &images_names_formatter.format()))?;
        Ok(())
    }

    async fn push_local_image(&self, image_name: &str, repository_uri_tag: &str) -> Result<()> {
        let push_output = tokio::process::Command::new("docker")
            .arg("push")
            .arg(repository_uri_tag)
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
        Ok(())
    }

    async fn delete_local_image(&self, image_name: &str, repository_uri_tag: &str) -> Result<()> {
        let delete_output = tokio::process::Command::new("docker")
            .arg("rmi")
            .arg(repository_uri_tag)
            .output()
            .await
            .into_diagnostic()?;
        if !delete_output.status.success() {
            return Err(miette::Error::msg(format!(
                "Failed to delete local image {}: {}",
                image_name,
                String::from_utf8_lossy(&delete_output.stderr)
            )));
        }
        Ok(())
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

#[derive(Clone)]
struct ImageData {
    image_name: String,
    repository_uri_tag: String,
}

struct ImagesNamesFormatter {
    names: Vec<String>,
}

impl ImagesNamesFormatter {
    fn new(names: Vec<String>) -> Self {
        let mut names: Vec<String> = names
            .into_iter()
            .map(|n| color_primary(n).to_string())
            .collect();
        names.sort();
        Self { names }
    }

    fn dim(&mut self) {
        for n in self.names.iter_mut() {
            *n = strip_ansi_escapes::strip_str(&mut *n)
                .dark_gray()
                .to_string();
        }
    }

    fn update_name(&mut self, name: &str, new_name: String) {
        for n in self.names.iter_mut() {
            if strip_ansi_escapes::strip_str(&mut *n).eq(name) {
                *n = new_name.clone();
            }
        }
    }

    fn format(&self) -> String {
        let images_len = self.names.len();
        let names = self.names.join(", ");
        if images_len > 1 {
            format!("images {}", &names)
        } else {
            format!("image {}", &names)
        }
    }
}
