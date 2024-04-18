use std::env::current_exe;
use std::process::{Command, Stdio};

use miette::IntoDiagnostic;
use miette::{miette, Context as _};
use rand::random;
use tracing::info;

use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::env::get_env_with_default;
use ockam_node::Context;

use crate::node::show::is_node_up;
use crate::node::CreateCommand;
use crate::util::api::TrustOpts;
use crate::CommandGlobalOpts;

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
    pub trust_opts: TrustOpts,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            tcp_listener_address: "127.0.0.1:0".to_string(),
            trust_opts: TrustOpts::default(),
        }
    }
}

pub async fn delete_all_nodes(opts: &CommandGlobalOpts, force: bool) -> miette::Result<()> {
    let nodes = opts.state.get_nodes().await?;
    let mut deletion_errors = Vec::new();
    for n in nodes {
        if let Err(e) = opts.state.delete_node(&n.name(), force).await {
            deletion_errors.push((n.name(), e));
        }
    }
    if !deletion_errors.is_empty() {
        return Err(miette!(
            "errors while deleting nodes: {:?}",
            deletion_errors
        ));
    }
    Ok(())
}

pub async fn initialize_default_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
) -> miette::Result<()> {
    if opts.state.get_default_node().await.is_err() {
        let cmd = CreateCommand::default();
        let node_name = cmd.name.clone();
        cmd.spawn_background_node(opts).await?;
        let mut node = BackgroundNodeClient::create_to_node(ctx, &opts.state, &node_name).await?;
        is_node_up(ctx, &mut node, true).await?;
    }
    Ok(())
}

/// A utility function to spawn a new node into foreground mode
#[allow(clippy::too_many_arguments)]
pub async fn spawn_node(opts: &CommandGlobalOpts, cmd: CreateCommand) -> miette::Result<()> {
    info!(
        "preparing to spawn a new node with name {} in the background",
        &cmd.name
    );

    let CreateCommand {
        skip_is_running_check,
        name,
        identity: identity_name,
        tcp_listener_address: address,
        launch_config,
        trust_opts,
        opentelemetry_context,
        ..
    } = cmd;
    let TrustOpts {
        project_name,
        authority_identity,
        authority_route,
        credential_scope,
    } = trust_opts;

    let mut args = vec![
        match opts.global_args.verbose {
            0 => "-vv".to_string(),
            v => format!("-{}", "v".repeat(v as usize)),
        },
        "node".to_string(),
        "create".to_string(),
        "--tcp-listener-address".to_string(),
        address.to_string(),
        "--foreground".to_string(),
        "--child-process".to_string(),
    ];

    if let Some(credential_scope) = credential_scope {
        args.push("--credential-scope".to_string());
        args.push(credential_scope)
    }

    if skip_is_running_check {
        args.push("--skip-is-running-check".to_string());
    }

    if !opts.terminal.is_tty() {
        args.push("--no-color".to_string());
    }

    if let Some(identity_name) = identity_name {
        args.push("--identity".to_string());
        args.push(identity_name);
    }

    if let Some(config) = launch_config {
        args.push("--launch-config".to_string());
        args.push(serde_json::to_string(&config).unwrap());
    }

    if let Some(project_name) = project_name {
        args.push("--project".to_string());
        args.push(project_name);
    }

    if let Some(authority_identity) = authority_identity {
        args.push("--authority-identity".to_string());
        args.push(authority_identity);
    }

    if let Some(authority_route) = authority_route {
        args.push("--authority-route".to_string());
        args.push(authority_route.to_string());
    }

    if let Some(opentelemetry_context) = opentelemetry_context {
        args.push("--opentelemetry-context".to_string());
        args.push(opentelemetry_context.to_string());
    }

    args.push(name.to_owned());

    run_ockam(args, opts.global_args.quiet).await
}

/// Run the ockam command line with specific arguments
pub async fn run_ockam(args: Vec<String>, quiet: bool) -> miette::Result<()> {
    info!("spawning a new process");

    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = current_exe().unwrap_or_else(|_| {
        get_env_with_default("OCKAM", "ockam".to_string())
            .unwrap()
            .into()
    });

    let stdio = || {
        if quiet {
            // If we're running in quiet mode, we don't need to propagate
            // the stdout/stderr to the child process
            Stdio::null()
        } else {
            // Otherwise, we need to inherit the stdout/stderr of the parent process
            // to see the output written in the child process
            Stdio::inherit()
        }
    };

    Command::new(ockam_exe)
        .args(args)
        .stdout(stdio())
        .stderr(stdio())
        .stdin(Stdio::null())
        .spawn()
        .into_diagnostic()
        .context("failed to spawn node")?;

    Ok(())
}
