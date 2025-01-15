use crate::node::CreateCommand;
use crate::run::parser::resource::utils::subprocess_stdio;
use crate::shared_args::TrustOpts;
use crate::{Command, CommandGlobalOpts};
use miette::IntoDiagnostic;
use miette::{miette, Context as _};
use ockam_core::env::get_env_with_default;
use ockam_node::Context;
use rand::random;
use std::env::current_exe;
use std::process::Stdio;
use tokio::process::{Child, Command as TokioCommand};
use tracing::info;

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
    if opts.state.is_using_in_memory_database()? {
        return Ok(());
    } else if opts.state.get_default_node().await.is_err() {
        let cmd = CreateCommand::default();
        cmd.run(ctx, opts.clone()).await?;
        opts.terminal.write_line("")?;
    } else {
        let node = opts.state.get_default_node().await?;
        if !node.is_running() {
            return Err(miette!("Default node exists but is not running"));
        }
    }
    Ok(())
}

/// Construct the argument list and re-execute the ockam
/// CLI in foreground mode to start the newly created node
pub fn spawn_node(opts: &CommandGlobalOpts, cmd: CreateCommand) -> miette::Result<Child> {
    info!(
        "preparing to spawn a new node with name {} in the background",
        &cmd.name
    );

    let CreateCommand {
        name,
        config_args,
        foreground_args,
        skip_is_running_check,
        tcp_listener_address,
        udp_listener_address,
        http_server,
        no_status_endpoint,
        status_endpoint_port,
        udp,
        launch_configuration,
        identity,
        trust_opts,
        opentelemetry_context,
        in_memory,
        tcp_callback_port,
    } = cmd;

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

    // global args
    if !opts.terminal.is_tty() {
        args.push("--no-color".to_string());
    }
    if opts.terminal.is_quiet() {
        args.push("--quiet".to_string());
    }
    if opts.global_args.no_input {
        args.push("--no-input".to_string());
    }
    if let Some(output_format) = opts.global_args.output_format.as_ref() {
        args.push("--output".to_string());
        args.push(output_format.to_string());
    }

    // config args
    if config_args.started_from_configuration {
        args.push("--started-from-configuration".to_string());
    }
    if let Some(enrollment_ticket) = config_args.enrollment_ticket {
        args.push("--enrollment-ticket".to_string());
        args.push(enrollment_ticket);
    }
    if let Some(configuration) = config_args.configuration {
        args.push("--configuration".to_string());
        args.push(configuration);
    }
    for (key, value) in config_args.variables {
        args.push("--variable".to_string());
        args.push(format!("{}={}", key, value));
    }

    if foreground_args.exit_on_eof {
        args.push("--exit-on-eof".to_string());
    }

    if skip_is_running_check {
        args.push("--skip-is-running-check".to_string());
    }

    // health check args
    if http_server {
        args.push("--http-server".to_string());
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

    if let Some(config) = launch_configuration {
        args.push("--launch-config".to_string());
        args.push(serde_json::to_string(&config).unwrap());
    }

    if let Some(identity_name) = identity {
        args.push("--identity".to_string());
        args.push(identity_name);
    }

    // trust opts
    if let Some(credential_scope) = trust_opts.credential_scope {
        args.push("--credential-scope".to_string());
        args.push(credential_scope)
    }
    if let Some(project_name) = trust_opts.project_name {
        args.push("--project".to_string());
        args.push(project_name);
    }
    if let Some(authority_identity) = trust_opts.authority_identity {
        args.push("--authority-identity".to_string());
        args.push(authority_identity.export_as_string().into_diagnostic()?);
    }
    if let Some(authority_route) = trust_opts.authority_route {
        args.push("--authority-route".to_string());
        args.push(authority_route.to_string());
    }

    if let Some(opentelemetry_context) = opentelemetry_context {
        args.push("--opentelemetry-context".to_string());
        args.push(opentelemetry_context.to_string());
    }

    if in_memory {
        args.push("--in-memory".to_string());
    }

    if let Some(tcp_callback_port) = tcp_callback_port {
        args.push("--tcp-callback-port".to_string());
        args.push(tcp_callback_port.to_string());
    }

    args.push(name.to_owned());

    run_ockam(args, opts.global_args.quiet)
}

/// Run the ockam command line with specific arguments
pub fn run_ockam(args: Vec<String>, quiet: bool) -> miette::Result<Child> {
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
        TokioCommand::new(ockam_exe)
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
            .context("failed to spawn node")
    }
}
