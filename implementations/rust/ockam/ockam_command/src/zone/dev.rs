use crate::cluster::common_args::{ClusterArg, HttpApiArgs};
use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node_command::InMemoryNodeCommand;
use crate::util::foreground_args::ForegroundArgs;
use crate::zone::common_args::{DockerBuildArgs, EnrollmentTicketConfigArg, ZoneNameOrConfigArg};
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::watcher::DirectoryWatcher;
use crate::zone::zone_config::{Container, Env, ZoneConfig};
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_api::CliState;
use ockam_node::Context;
use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

const LONG_ABOUT: &str = include_str!("./static/dev/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/dev/after_long_help.txt");

/// Run an Autonomy zone locally for development
#[derive(Clone, Debug, Args, Default)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DevCommand {
    #[command(flatten)]
    pub cluster: ClusterArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    #[command(flatten)]
    pub docker_build: DockerBuildArgs,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    #[command(flatten)]
    pub enrollment_ticket: EnrollmentTicketConfigArg,

    /// Port to expose the app's HTTP server on localhost
    #[arg(long, short = 'p', default_value = "8000")]
    pub port: u16,

    /// Port for the gateway portal (0 = ephemeral)
    #[arg(long, default_value = "8080")]
    pub gateway_port: u16,

    /// Watch for file changes and auto-reload containers
    #[arg(long, short = 'w')]
    pub watch: bool,

    /// Remove containers and network when done
    #[arg(long)]
    pub rm: bool,

    /// Skip creating gateway portal (use existing gateway connection)
    #[arg(long)]
    pub no_gateway_portal: bool,

    /// Gateway URL to use (if --no-gateway-portal)
    #[arg(long, env = "AUTONOMY_EXTERNAL_APIS_GATEWAY_URL")]
    pub gateway_url: Option<String>,

    /// Gateway API key to use
    #[arg(long, env = "AUTONOMY_EXTERNAL_APIS_GATEWAY_API_KEY")]
    pub gateway_api_key: Option<String>,

    /// Docker network name for containers
    #[arg(long, default_value = "autonomy-dev")]
    pub network: String,

    /// Additional environment variables (KEY=VALUE)
    #[arg(long, short = 'e')]
    pub env: Vec<String>,

    /// Additional volume mounts (HOST:CONTAINER)
    #[arg(long, short = 'v')]
    pub volume: Vec<String>,

    /// Container runtime to use (docker or podman)
    #[arg(long, default_value = "docker")]
    pub runtime: String,

    /// Run in background mode (don't wait for Ctrl+C)
    #[arg(long)]
    pub background: bool,
}

impl DevCommand {
    pub fn name(&self) -> String {
        "zone dev".into()
    }
}

#[derive(Clone)]
struct DevNodeCommand {
    opts: CommandGlobalOpts,
    command: DevCommand,
    gateway_inlet_port: u16,
}

#[async_trait]
impl InMemoryNodeCommand for DevNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;

        // Get cluster
        let cluster = self.command.cluster.get_cluster(ctx, &node).await?;

        // Parse zone config
        let zone_config = self.command.zone.zone_config()?;
        let zone_name = &zone_config.name;

        self.opts.terminal.write_line(fmt_log!(
            "Starting local development for zone {} in cluster {}...\n",
            color_primary(zone_name),
            color_primary(&cluster),
        ))?;

        // Create Docker network
        self.command.create_docker_network(&self.opts).await?;

        // Build Docker images
        self.command
            .build_images(&self.opts, &zone_config)
            .await?;

        // Determine gateway URL and token based on whether we're creating a portal
        let (gateway_url, gateway_token_str, node_handle) = if self.command.no_gateway_portal {
            // Use existing gateway connection (no portal)
            let url = self.command.gateway_url.clone()
                .unwrap_or_else(|| "http://localhost:8080".to_string());

            // If gateway API key is provided, use it directly without fetching from orchestrator
            let token = if let Some(key) = &self.command.gateway_api_key {
                self.opts.terminal.write_line(fmt_log!(
                    "Using provided gateway API key"
                ))?;
                key.clone()
            } else {
                // Try to fetch gateway token for the zone, fall back to dev token if zone doesn't exist
                self.opts.terminal.write_line(fmt_log!(
                    "Fetching gateway token for API authentication..."
                ))?;
                let gateway_token = match api_client
                    .create_gateway_token(ctx, Some(&cluster), zone_name)
                    .await
                {
                    Ok(token) => token,
                    Err(e) if Self::is_zone_not_found_error(&e) => {
                        self.opts.terminal.write_line(fmt_warn!(
                            "Zone '{}' does not exist. Using development token instead.\n",
                            zone_name
                        ))?;
                        self.opts.terminal.write_line(fmt_log!(
                            "Tip: Create the zone with: {} zone create {}\n",
                            color_primary("autonomy"),
                            color_primary(zone_name)
                        ))?;
                        api_client.create_dev_token(ctx, Some(&cluster)).await?
                    }
                    Err(e) => return Err(e),
                };
                self.opts.terminal.write_line(fmt_ok!(
                    "Gateway token acquired (expires in {} seconds)",
                    gateway_token.expires_in
                ))?;
                gateway_token.token
            };

            self.opts.terminal.write_line(fmt_log!(
                "Using existing gateway at {}",
                color_primary(&url),
            ))?;
            (url, token, None)
        } else {
            // Try to get enrollment ticket for the gateway portal node
            // If zone doesn't exist, fall back to dev token mode
            let enrollment_ticket_result = self
                .command
                .enrollment_ticket
                .get(ctx, &*api_client, &cluster, zone_name, None)
                .await;

            match enrollment_ticket_result {
                Ok(enrollment_ticket) => {
                    // Zone exists - use full portal flow
                    // Get gateway token for API authentication
                    self.opts.terminal.write_line(fmt_log!(
                        "Fetching gateway token for API authentication..."
                    ))?;

                    let gateway_token = api_client
                        .create_gateway_token(ctx, Some(&cluster), zone_name)
                        .await?;

                    self.opts.terminal.write_line(fmt_ok!(
                        "Gateway token acquired (expires in {} seconds)",
                        gateway_token.expires_in
                    ))?;

                    // Start the gateway portal node in a background task
                    // This creates a TCP inlet that tunnels to the gateway via Ockam relay
                    let relay_name = "gateway".to_string();
                    let inlet_addr = format!("127.0.0.1:{}", self.gateway_inlet_port);

                    self.opts.terminal.write_line(fmt_log!(
                        "Creating gateway portal at {} via relay {}...",
                        color_primary(&inlet_addr),
                        color_primary(&relay_name),
                    ))?;

                    let node_config = serde_json::json!({
                        "tcp-inlet": {
                            "from": format!("tcp://{}", inlet_addr),
                            "to": "gateway",
                            "via": relay_name
                        }
                    });

                    let in_memory = true;
                    let is_verbose = self.opts.global_args.verbose > 0;

                    let node_cmd = crate::node::create::CreateCommand {
                        name: node_config.to_string(),
                        config_args: ConfigArgs {
                            enrollment_ticket: Some(enrollment_ticket.clone()),
                            ..Default::default()
                        },
                        foreground_args: ForegroundArgs {
                            foreground: true,
                            no_ctrlc_handler: true,
                            ..Default::default()
                        },
                        in_memory,
                        suppress_notifications: !is_verbose,
                        ..Default::default()
                    };

                    let opts = self.opts.clone();

                    // Spawn the gateway portal node in the background
                    let handle = tokio::spawn(async move {
                        let mut opts_inner = opts.clone();
                        opts_inner.state = Arc::new(CliState::new(in_memory).await?);
                        tokio::select! {
                            _ = DirectoryWatcher::wait_for_message() => Ok(()),
                            res = node_cmd.run(node.ctx(), opts_inner) => res,
                        }
                    });

                    // Wait for the gateway inlet to be ready
                    let url = format!("http://127.0.0.1:{}", self.gateway_inlet_port);
                    self.wait_for_gateway_ready(&url).await?;

                    self.opts.terminal.write_line(fmt_ok!(
                        "Gateway portal ready at {}\n",
                        color_primary(&url),
                    ))?;

                    (url, gateway_token.token.clone(), Some(handle))
                }
                Err(e) if Self::is_zone_not_found_error(&e) => {
                    // Zone doesn't exist - fall back to dev token mode
                    self.opts.terminal.write_line(fmt_warn!(
                        "Zone '{}' does not exist. Running in development mode.\n",
                        zone_name
                    ))?;
                    self.opts.terminal.write_line(fmt_log!(
                        "Development mode uses a temporary token tied to your account.\n\
                         To create the zone for production: {} zone create {}\n",
                        color_primary("autonomy"),
                        color_primary(zone_name)
                    ))?;

                    // Get dev token (doesn't require zone)
                    self.opts.terminal.write_line(fmt_log!(
                        "Fetching development token..."
                    ))?;
                    let gateway_token = api_client
                        .create_dev_token(ctx, Some(&cluster))
                        .await?;
                    self.opts.terminal.write_line(fmt_ok!(
                        "Development token acquired (expires in {} seconds)",
                        gateway_token.expires_in
                    ))?;

                    // In dev mode without a zone, we can't create the portal
                    // because we don't have an enrollment ticket. Use --no-gateway-portal flow.
                    let url = self.command.gateway_url.clone()
                        .unwrap_or_else(|| "http://localhost:8080".to_string());

                    self.opts.terminal.write_line(fmt_warn!(
                        "Gateway portal not available without zone. Using direct gateway at {}.\n\
                         You may need to set up port-forwarding to the gateway.\n",
                        color_primary(&url)
                    ))?;

                    (url, gateway_token.token, None)
                }
                Err(e) => return Err(e),
            }
        };



        let zone_config_clone = zone_config.clone();

        // Run containers with gateway URL
        self.command
            .run_containers(&self.opts, &zone_config_clone, &gateway_url, &gateway_token_str)
            .await?;

        self.opts.terminal.write_line(fmt_ok!(
            "App running at http://localhost:{}\n",
            self.command.port
        ))?;

        // Watch for changes or wait for Ctrl+C
        if self.command.watch {
            self.opts.terminal.write_line(fmt_log!(
                "Watching for file changes... Press Ctrl+C to stop."
            ))?;
            DirectoryWatcher::init()?;
            self.command
                .watch_and_reload(&self.opts, &zone_config_clone, &gateway_url, &gateway_token_str)
                .await?;
        } else if !self.command.background {
            self.opts.terminal.write_line(fmt_log!(
                "Press Ctrl+C to stop."
            ))?;
            tokio::select! {
                _ = ZoneCtrlcHandler::wait_for_message() => {
                    debug!("Received Ctrl+C, shutting down");
                }
            }
        }

        // Cleanup if requested
        if self.command.rm {
            self.opts.terminal.write_line(fmt_log!(
                "\nCleaning up containers and network..."
            ))?;
            self.command.cleanup(&self.opts, &zone_config_clone).await?;
            self.opts.terminal.write_line(fmt_ok!("Cleanup complete"))?;
        }

        // Abort the gateway portal node if we created one
        if let Some(handle) = node_handle {
            handle.abort();
        }

        Ok(())
    }
}

impl DevNodeCommand {
    /// Check if an error indicates that the zone was not found (404).
    ///
    /// This is used to detect when a zone doesn't exist so we can fall back
    /// to dev token mode with helpful guidance.
    fn is_zone_not_found_error(err: &miette::Report) -> bool {
        let err_string = format!("{:?}", err);
        // Check for HTTP 404 or "does not exist" message from provisioner
        err_string.contains("HTTP 404")
            || err_string.contains("404")
            || err_string.contains("does not exist")
            || err_string.contains("not found")
            || err_string.contains("Not Found")
    }

    /// Wait for the gateway inlet to be ready by polling the health endpoint
    async fn wait_for_gateway_ready(&self, gateway_url: &str) -> miette::Result<()> {
        let max_retries = 120; // 120 * 500ms = 60 seconds
        let delay = Duration::from_millis(500);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .into_diagnostic()?;

        let health_url = format!("{}/health", gateway_url);

        for i in 0..max_retries {
            match client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    debug!(
                        "Gateway ready at {} after {} attempts",
                        gateway_url,
                        i + 1
                    );
                    return Ok(());
                }
                Ok(response) => {
                    debug!(
                        "Gateway at {} returned status {}, retrying...",
                        gateway_url,
                        response.status()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    debug!("Gateway at {} not ready yet: {}, retrying...", gateway_url, e);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(miette::miette!(
            "Timeout waiting for gateway portal to be ready at {}. \
            Make sure the gateway-outlet pod is deployed in your cluster.",
            gateway_url
        ))
    }
}

#[async_trait]
impl Command for DevCommand {
    const NAME: &'static str = "zone dev";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        // Find an available port for the gateway inlet
        let gateway_inlet_port = if self.gateway_port == 0 {
            find_available_port().await?
        } else {
            self.gateway_port
        };

        let command = DevNodeCommand {
            opts: opts.clone(),
            command: self,
            gateway_inlet_port,
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

/// Find an available port by binding to port 0
async fn find_available_port() -> miette::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .into_diagnostic()?;
    let port = listener.local_addr().into_diagnostic()?.port();
    drop(listener);
    Ok(port)
}

impl DevCommand {
    /// Create the Docker network for local development
    async fn create_docker_network(&self, opts: &CommandGlobalOpts) -> miette::Result<()> {
        debug!("Creating Docker network: {}", self.network);

        // Check if network already exists
        let check_output = tokio::process::Command::new(&self.runtime)
            .args(["network", "inspect", &self.network])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .into_diagnostic()?;

        if check_output.success() {
            debug!("Network {} already exists", self.network);
            return Ok(());
        }

        // Create network
        let output = tokio::process::Command::new(&self.runtime)
            .args(["network", "create", &self.network])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .into_diagnostic()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "already exists" errors (race condition)
            if !stderr.contains("already exists") {
                return Err(miette::miette!(
                    "Failed to create Docker network '{}': {}",
                    self.network,
                    stderr
                ));
            }
        }

        opts.terminal.write_line(fmt_log!(
            "Created Docker network: {}",
            color_primary(&self.network)
        ))?;

        Ok(())
    }

    /// Build Docker images for all pods in the zone config
    async fn build_images(
        &self,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
    ) -> miette::Result<()> {
        opts.terminal.write_line(fmt_log!("Building Docker images..."))?;

        for pod in &zone_config.pods {
            for container in &pod.containers {
                let image_name = &container.image;

                // Look for Dockerfile in images/<image_name>/Dockerfile
                let dockerfile_path = format!("images/{}/Dockerfile", image_name);
                let context_path = format!("images/{}", image_name);

                if !std::path::Path::new(&dockerfile_path).exists() {
                    debug!(
                        "No Dockerfile found at {}, skipping build for {}",
                        dockerfile_path, image_name
                    );
                    continue;
                }

                let local_image_name = format!("autonomy-dev-{}", image_name);
                opts.terminal.write_line(fmt_log!(
                    "  Building image: {}",
                    color_primary(&local_image_name)
                ))?;

                let mut args = vec![
                    "build".to_string(),
                    "-t".to_string(),
                    local_image_name.clone(),
                    "-f".to_string(),
                    dockerfile_path,
                ];

                if self.docker_build.no_cache {
                    args.push("--no-cache".to_string());
                }

                if !self.docker_build.no_pull {
                    args.push("--pull".to_string());
                }

                args.push(context_path);

                // Use DOCKER_BUILDKIT=0 for better compatibility across different
                // Docker/Podman setups - avoids issues with buildx caching
                let output = tokio::process::Command::new(&self.runtime)
                    .args(&args)
                    .env("DOCKER_BUILDKIT", "0")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await
                    .into_diagnostic()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(miette::miette!(
                        "Failed to build image '{}': {}",
                        image_name,
                        stderr
                    ));
                }

                info!("Built image: {}", local_image_name);
            }
        }

        opts.terminal.write_line(fmt_ok!("Images built successfully\n"))?;
        Ok(())
    }

    /// Run containers for all pods in the zone config
    async fn run_containers(
        &self,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
        gateway_url: &str,
        gateway_token: &str,
    ) -> miette::Result<()> {
        opts.terminal.write_line(fmt_log!("Starting containers..."))?;

        // Determine the gateway URL for containers
        // If it's localhost, use host.docker.internal or host.containers.internal
        let gateway_url_for_container = if gateway_url.contains("localhost") || gateway_url.contains("127.0.0.1") {
            let gateway_host = if self.runtime == "podman" {
                "host.containers.internal"
            } else {
                "host.docker.internal"
            };
            gateway_url.replace("localhost", gateway_host).replace("127.0.0.1", gateway_host)
        } else {
            gateway_url.to_string()
        };

        for pod in &zone_config.pods {
            for container in &pod.containers {
                let image_name = &container.image;
                let container_name = format!("autonomy-dev-{}-{}", pod.name, container.name);

                // Remove existing container if it exists
                let _ = tokio::process::Command::new(&self.runtime)
                    .args(["rm", "-f", &container_name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                let mut args = vec![
                    "run".to_string(),
                    "-d".to_string(),
                    "--name".to_string(),
                    container_name.clone(),
                    "--network".to_string(),
                    self.network.clone(),
                ];

                // Add port mapping for main container
                if container.name == "main" {
                    args.extend(["-p".to_string(), format!("{}:8000", self.port)]);
                }

                // Add gateway environment variables
                args.extend([
                    "-e".to_string(),
                    "AUTONOMY_USE_EXTERNAL_APIS_GATEWAY=1".to_string(),
                ]);
                args.extend([
                    "-e".to_string(),
                    format!("AUTONOMY_EXTERNAL_APIS_GATEWAY_URL={}", gateway_url_for_container),
                ]);
                args.extend([
                    "-e".to_string(),
                    format!("AUTONOMY_EXTERNAL_APIS_GATEWAY_API_KEY={}", gateway_token),
                ]);

                // Add container-specific env vars from autonomy.yaml
                self.add_container_env_vars(&mut args, container);

                // Add user-specified env vars
                for env in &self.env {
                    args.extend(["-e".to_string(), env.clone()]);
                }

                // Add volume mounts
                for vol in &self.volume {
                    args.extend(["-v".to_string(), vol.clone()]);
                }

                // Determine which image to use
                let dockerfile_path = format!("images/{}/Dockerfile", image_name);
                let image_to_use = if std::path::Path::new(&dockerfile_path).exists() {
                    format!("autonomy-dev-{}", image_name)
                } else {
                    // Use the image name directly (pre-built image)
                    image_name.clone()
                };

                args.push(image_to_use);

                opts.terminal.write_line(fmt_log!(
                    "  Starting container: {}",
                    color_primary(&container_name)
                ))?;

                let output = tokio::process::Command::new(&self.runtime)
                    .args(&args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await
                    .into_diagnostic()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(miette::miette!(
                        "Failed to start container '{}': {}",
                        container_name,
                        stderr
                    ));
                }

                info!("Started container: {}", container_name);
            }
        }

        opts.terminal.write_line(fmt_ok!("Containers started successfully"))?;
        Ok(())
    }

    /// Watch for file changes and reload containers
    async fn watch_and_reload(
        &self,
        opts: &CommandGlobalOpts,
        zone_config: &ZoneConfig,
        gateway_url: &str,
        gateway_token: &str,
    ) -> miette::Result<()> {
        loop {
            tokio::select! {
                result = DirectoryWatcher::wait_for_message() => {
                    match result {
                        Ok(message) => {
                            opts.terminal.write_line(fmt_warn!("\n{}", message))?;
                            opts.terminal.write_line(fmt_log!("Rebuilding and restarting containers..."))?;

                            // Stop existing containers
                            self.stop_containers(zone_config).await?;

                            // Rebuild images
                            self.build_images(opts, zone_config).await?;

                            // Restart containers
                            self.run_containers(opts, zone_config, gateway_url, gateway_token).await?;

                            opts.terminal.write_line(fmt_ok!(
                                "App reloaded at http://localhost:{}\n",
                                self.port
                            ))?;
                        }
                        Err(e) => {
                            warn!("Error watching for changes: {}", e);
                            break;
                        }
                    }
                }
                _ = ZoneCtrlcHandler::wait_for_message() => {
                    debug!("Received Ctrl+C, shutting down");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Stop all containers for the zone
    async fn stop_containers(&self, zone_config: &ZoneConfig) -> miette::Result<()> {
        for pod in &zone_config.pods {
            for container in &pod.containers {
                let container_name = format!("autonomy-dev-{}-{}", pod.name, container.name);

                let _ = tokio::process::Command::new(&self.runtime)
                    .args(["stop", &container_name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                let _ = tokio::process::Command::new(&self.runtime)
                    .args(["rm", "-f", &container_name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
            }
        }
        Ok(())
    }

    /// Cleanup containers and network
    async fn cleanup(&self, _opts: &CommandGlobalOpts, zone_config: &ZoneConfig) -> miette::Result<()> {
        // Stop and remove containers
        self.stop_containers(zone_config).await?;

        // Remove network
        let _ = tokio::process::Command::new(&self.runtime)
            .args(["network", "rm", &self.network])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        Ok(())
    }

    /// Add container-specific environment variables from autonomy.yaml
    fn add_container_env_vars(&self, args: &mut Vec<String>, container: &Container) {
        if let Some(env) = &container.env {
            match env {
                Env::ListOfMaps(list) => {
                    for map in list {
                        // Handle Kubernetes-style env format: {name: KEY, value: VALUE}
                        if let (Some(Value::String(name)), Some(Value::String(value))) =
                            (map.get("name"), map.get("value"))
                        {
                            args.extend(["-e".to_string(), format!("{}={}", name, value)]);
                        }
                        // Handle simple key: value format
                        else if map.len() == 1 {
                            if let Some((key, Value::String(value))) = map.iter().next() {
                                args.extend(["-e".to_string(), format!("{}={}", key, value)]);
                            }
                        }
                    }
                }
                Env::MapOfKeyValues(map) => {
                    for (key, value) in map {
                        args.extend(["-e".to_string(), format!("{}={}", key, value)]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let cmd = DevCommand::default();
        assert_eq!(cmd.name(), "zone dev");
    }

    #[test]
    fn test_default_values() {
        let cmd = DevCommand::default();
        // Default trait gives default values, clap would give the specified defaults
        assert!(!cmd.watch);
        assert!(!cmd.rm);
        assert!(!cmd.background);
    }
}
