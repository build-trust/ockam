use crate::cluster::common_args::{ClusterArg, HttpApiArgs};
use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node_command::InMemoryNodeCommand;
use crate::util::foreground_args::ForegroundArgs;
use crate::zone::common_args::{EnrollmentTicketConfigArg, ZoneNameOrConfigArg};
use crate::zone::ctrlc::ZoneCtrlcHandler;
use crate::zone::watcher::DirectoryWatcher;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use futures_util::StreamExt;
use miette::IntoDiagnostic;
use ockam::transport::SchemeHostnamePort;
use ockam_api::nodes::InMemoryNode;
use ockam_api::CliState;

use ockam_node::Context;
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

const LONG_ABOUT: &str = include_str!("./static/logs/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/logs/after_long_help.txt");

/// Stream logs from containers in a deployed zone
#[derive(Clone, Debug, Args, Default)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct LogsCommand {
    #[command(flatten)]
    pub cluster: ClusterArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    /// Pod name to get logs from (supports partial match, e.g., "main-pod")
    #[arg(long, short = 'p', default_value = "main-pod")]
    pub pod: String,

    /// Container name to get logs from
    #[arg(long, short = 'c', default_value = "main")]
    pub container: String,

    /// Follow log output (stream continuously)
    #[arg(long, short = 'f', default_value = "true")]
    pub follow: bool,

    /// Disable following log output
    #[arg(long, conflicts_with = "follow")]
    pub no_follow: bool,

    /// Number of lines to show from end of logs (-1 for all)
    #[arg(long, default_value = "-1")]
    pub tail: i32,

    /// Show logs newer than relative duration (e.g., 5s, 2m, 3h)
    #[arg(long)]
    pub since: Option<String>,

    /// Include timestamps in output
    #[arg(long, short = 't')]
    pub timestamps: bool,

    /// List available pods and containers instead of streaming logs
    #[arg(long, short = 'l')]
    pub list: bool,

    /// Disable automatic reconnection on container restart
    #[arg(long)]
    pub no_reconnect: bool,

    // == Node Options ==
    #[command(flatten)]
    pub enrollment_ticket: EnrollmentTicketConfigArg,

    #[command(flatten)]
    pub http_api: HttpApiArgs,
}

impl LogsCommand {
    pub fn name(&self) -> String {
        "zone logs".into()
    }

    /// Check if follow is enabled (considering --no-follow flag)
    fn should_follow(&self) -> bool {
        self.follow && !self.no_follow
    }
}

#[derive(Clone)]
struct LogsNodeCommand {
    opts: CommandGlobalOpts,
    command: LogsCommand,
    inlet_port: u16,
}

#[async_trait]
impl InMemoryNodeCommand for LogsNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = self.command.cluster.get_cluster(ctx, &node).await?;
        let zone_config = self.command.zone.zone_config()?;
        let zone_name = zone_config.name;
        let enrollment_ticket = self
            .command
            .enrollment_ticket
            .get(ctx, &*api_client, &cluster, &zone_name, None)
            .await?;

        // Create inlet to logs outlet on logs-pod
        let relay_name = format!("{}-{}-{}", cluster, zone_name, "logs-pod");

        // Use the specified port for the inlet
        let inlet_from =
            SchemeHostnamePort::from_str(&format!("tcp://127.0.0.1:{}", self.inlet_port))?;
        let inlet_addr = format!("127.0.0.1:{}", self.inlet_port);

        let node_config = serde_json::json!({
            "tcp-inlet": {
                "from": inlet_from.to_string(),
                "to": "logs",
                "via": relay_name
            }
        });

        let in_memory = true;
        // Suppress identity/vault creation messages unless verbose mode is enabled
        let is_verbose = self.opts.global_args.verbose > 0;

        let node_cmd = crate::node::create::CreateCommand {
            name: node_config.to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(enrollment_ticket),
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

        let command = self.command.clone();
        let opts = self.opts.clone();

        // Spawn the node creation in a background task, moving node into the closure
        let handle = tokio::spawn(async move {
            let mut opts = opts;
            opts.state = Arc::new(CliState::new(in_memory).await?);
            tokio::select! {
                _ = DirectoryWatcher::wait_for_message() => Ok(()),
                res = node_cmd.run(node.ctx(), opts) => res,
            }
        });

        // Wait for the inlet to be ready (polls HTTP endpoint)
        let wait_result = wait_for_inlet_ready(&inlet_addr).await;

        // If wait failed, abort and return error
        if let Err(e) = wait_result {
            handle.abort();
            return Err(e);
        }

        // Now stream logs or list containers
        let result = if command.list {
            list_containers_and_print(&inlet_addr).await
        } else {
            // Stream logs with Ctrl+C handling
            tokio::select! {
                result = stream_logs(&inlet_addr, &command) => {
                    result
                }
                _ = ZoneCtrlcHandler::wait_for_message() => {
                    debug!("Received Ctrl+C, shutting down");
                    Ok(())
                }
            }
        };

        // Cleanup: abort the node handle
        handle.abort();

        result
    }
}

#[async_trait]
impl Command for LogsCommand {
    const NAME: &'static str = "zone logs";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        // Find an available port for the inlet
        let inlet_port = find_available_port().await?;

        let node_command = LogsNodeCommand {
            opts: opts.clone(),
            command: self,
            inlet_port,
        };

        // Execute the command using the InMemoryNodeCommand pattern
        node_command.execute(ctx, opts.state.clone()).await?;

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

/// Wait for the inlet to be ready by polling for HTTP connectivity to the logs server
async fn wait_for_inlet_ready(inlet_addr: &str) -> miette::Result<()> {
    let max_retries = 240; // 240 * 500ms = 120 seconds
    let delay = Duration::from_millis(500);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .into_diagnostic()?;

    let url = format!("http://{}/containers", inlet_addr);

    for i in 0..max_retries {
        // Try to make an HTTP request to verify the inlet is fully functional
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                debug!(
                    "Inlet ready at {} after {} attempts (HTTP check passed)",
                    inlet_addr,
                    i + 1
                );
                return Ok(());
            }
            Ok(response) => {
                debug!(
                    "Inlet at {} returned status {}, retrying...",
                    inlet_addr,
                    response.status()
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                debug!("Inlet at {} not ready yet: {}, retrying...", inlet_addr, e);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(miette::miette!(
        "Timeout waiting for inlet to be ready at {}. The logs server may not be reachable.",
        inlet_addr
    ))
}

/// List available pods and containers from the logs server
async fn list_containers(inlet_addr: &str) -> miette::Result<HashMap<String, Vec<String>>> {
    let url = format!("http://{}/containers", inlet_addr);
    debug!("Fetching containers from: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .into_diagnostic()?;

    let response = client
        .get(&url)
        .send()
        .await
        .into_diagnostic()
        .map_err(|e| miette::miette!("Failed to connect to logs server: {}", e))?;

    if !response.status().is_success() {
        return Err(miette::miette!(
            "Logs server returned error: {}",
            response.status()
        ));
    }

    let containers: HashMap<String, Vec<String>> = response.json().await.into_diagnostic()?;

    Ok(containers)
}

/// List containers and print them to stdout
async fn list_containers_and_print(inlet_addr: &str) -> miette::Result<()> {
    let containers = list_containers(inlet_addr).await?;

    println!("Available pods and containers:");
    for (pod_name, container_list) in containers.iter() {
        println!("  {}:", pod_name);
        for container in container_list {
            println!("    - {}", container);
        }
    }

    Ok(())
}

/// Find the full pod name from a prefix (pods have random suffixes like main-pod-767c44cd4b-6bxz6)
async fn find_full_pod_name(inlet_addr: &str, pod_prefix: &str) -> miette::Result<String> {
    let containers = list_containers(inlet_addr).await?;

    for pod_name in containers.keys() {
        if pod_name.starts_with(pod_prefix) {
            return Ok(pod_name.clone());
        }
    }

    // If no match found, list available pods in error message
    let available: Vec<&String> = containers.keys().collect();
    Err(miette::miette!(
        "Pod not found matching '{}'. Available pods: {:?}",
        pod_prefix,
        available
    ))
}

/// Stream logs from the logs server to stdout
async fn stream_logs(inlet_addr: &str, opts: &LogsCommand) -> miette::Result<()> {
    // First, find the full pod name
    let full_pod_name = find_full_pod_name(inlet_addr, &opts.pod).await?;
    debug!("Found full pod name: {}", full_pod_name);

    // Build the URL with query parameters
    let follow = opts.should_follow();
    let mut url = format!(
        "http://{}/pods/{}/containers/{}?follow={}&timestamps={}&auto_reconnect={}",
        inlet_addr, full_pod_name, opts.container, follow, opts.timestamps, !opts.no_reconnect
    );

    if opts.tail >= 0 {
        url.push_str(&format!("&tail={}", opts.tail));
    }
    if let Some(since) = &opts.since {
        url.push_str(&format!("&since={}", since));
    }

    debug!("Streaming logs from: {}", url);

    // Use reqwest with streaming support
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .into_diagnostic()
        .map_err(|e| miette::miette!("Failed to connect to logs server: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(miette::miette!(
            "Logs server returned error {}: {}",
            status,
            body
        ));
    }

    // Stream the response body to stdout using bytes_stream
    let mut stream = response.bytes_stream();
    let mut stdout = std::io::stdout();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                stdout.write_all(&bytes).into_diagnostic()?;
                stdout.flush().into_diagnostic()?;
            }
            Err(e) => {
                debug!("Error reading log stream: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a LogsCommand with clap's default values applied
    fn create_cmd_with_defaults() -> LogsCommand {
        LogsCommand {
            pod: "main-pod".to_string(),
            container: "main".to_string(),
            follow: true,
            no_follow: false,
            tail: -1,
            timestamps: false,
            list: false,
            no_reconnect: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_default_values() {
        let cmd = create_cmd_with_defaults();
        assert_eq!(cmd.pod, "main-pod");
        assert_eq!(cmd.container, "main");
        assert!(cmd.follow);
        assert!(!cmd.no_follow);
        assert_eq!(cmd.tail, -1);
        assert!(!cmd.timestamps);
        assert!(!cmd.list);
        assert!(!cmd.no_reconnect);
    }

    #[test]
    fn test_should_follow() {
        let mut cmd = create_cmd_with_defaults();
        assert!(cmd.should_follow());

        cmd.no_follow = true;
        assert!(!cmd.should_follow());

        cmd.no_follow = false;
        cmd.follow = false;
        assert!(!cmd.should_follow());
    }

    #[test]
    fn test_name() {
        let cmd = create_cmd_with_defaults();
        assert_eq!(cmd.name(), "zone logs");
    }
}
