use miette::Context as _;
use miette::IntoDiagnostic;
use ockam_core::env::get_env_with_default;
use ockam_node::Context;
use rand::random;
use std::env::current_exe;
use std::os::unix::prelude::CommandExt;
use std::process::{Command, Stdio};
use tracing::info;

use crate::node::show::wait_until_node_is_up;
use crate::node::CreateCommand;
use crate::run::parser::resource::utils::subprocess_stdio;
use crate::shared_args::TrustOpts;
use crate::{Command as CommandTrait, CommandGlobalOpts};

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
    pub udp_listener_address: String,
    pub trust_opts: TrustOpts,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            tcp_listener_address: "127.0.0.1:0".to_string(),
            udp_listener_address: "127.0.0.1:0".to_string(),
            trust_opts: TrustOpts::default(),
        }
    }
}

pub async fn initialize_default_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
) -> miette::Result<()> {
    if opts.state.get_default_node().await.is_err() {
        let cmd = CreateCommand::default();
        cmd.async_run(ctx, opts.clone()).await?;
        opts.terminal.write_line("")?;
    }
    let node_name = opts.state.get_default_node().await?;
    wait_until_node_is_up(ctx, &opts.state, node_name.name()).await?;
    Ok(())
}

/// Construct the argument list and re-execute the ockam
/// CLI in foreground mode to start the newly created node
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
        tcp_listener_address,
        udp_listener_address,
        no_status_endpoint,
        status_endpoint_port,
        udp,
        launch_configuration,
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
        tcp_listener_address.to_string(),
        "--udp-listener-address".to_string(),
        udp_listener_address.to_string(),
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

    if let Some(config) = launch_configuration {
        args.push("--launch-config".to_string());
        args.push(serde_json::to_string(&config).unwrap());
    }

    if let Some(project_name) = project_name {
        args.push("--project".to_string());
        args.push(project_name);
    }

    if let Some(authority_identity) = authority_identity {
        args.push("--authority-identity".to_string());
        args.push(authority_identity.export_as_string().into_diagnostic()?);
    }

    if let Some(authority_route) = authority_route {
        args.push("--authority-route".to_string());
        args.push(authority_route.to_string());
    }

    if let Some(opentelemetry_context) = opentelemetry_context {
        args.push("--opentelemetry-context".to_string());
        args.push(opentelemetry_context.to_string());
    }

    if no_status_endpoint {
        args.push("--no-status-endpoint".to_string());
    }

    if let Some(status_endpoint_port) = status_endpoint_port {
        args.push("--status-endpoint-port".to_string());
        args.push(status_endpoint_port.to_string());
    }

    if udp {
        args.push("--udp".to_string());
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

    unsafe {
        Command::new(ockam_exe)
            .args(args)
            .stdout(subprocess_stdio(quiet))
            .stderr(subprocess_stdio(quiet))
            .stdin(Stdio::null())
            // This unsafe block will only panic if the closure panics, which shouldn't happen
            .pre_exec(|| {
                // Detach the process from the parent
                nix::unistd::setsid().map_err(std::io::Error::from)?;
                Ok(())
            })
            .spawn()
            .into_diagnostic()
            .context("failed to spawn node")?;
    }

    Ok(())
}
