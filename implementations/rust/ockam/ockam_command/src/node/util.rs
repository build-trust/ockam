use std::env::current_exe;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use miette::IntoDiagnostic;
use miette::{miette, Context as _};
use rand::random;

use ockam_api::cli_state::NamedTrustContext;
use ockam_api::nodes::BackgroundNode;
use ockam_core::env::get_env_with_default;
use ockam_node::Context;

use crate::node::background::spawn_background_node;
use crate::node::show::is_node_up;
use crate::node::CreateCommand;
use crate::util::api::TrustContextOpts;
use crate::CommandGlobalOpts;

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
    pub trust_context_opts: TrustContextOpts,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            tcp_listener_address: "127.0.0.1:0".to_string(),
            trust_context_opts: TrustContextOpts::default(),
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
        let node_name = cmd.node_name.clone();
        spawn_background_node(opts, cmd).await?;
        let mut node = BackgroundNode::create_to_node(ctx, &opts.state, &node_name).await?;
        is_node_up(ctx, &mut node, true).await?;
    }
    Ok(())
}

/// A utility function to spawn a new node into foreground mode
#[allow(clippy::too_many_arguments)]
pub async fn spawn_node(
    opts: &CommandGlobalOpts,
    name: &str,
    identity_name: &Option<String>,
    address: &str,
    trusted_identities: Option<&String>,
    trusted_identities_file: Option<&PathBuf>,
    reload_from_trusted_identities_file: Option<&PathBuf>,
    launch_config: Option<String>,
    credential: Option<&String>,
    trust_context: Option<&NamedTrustContext>,
    project_name: Option<String>,
    logging_to_file: bool,
) -> miette::Result<()> {
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

    if logging_to_file || !opts.terminal.is_tty() {
        args.push("--no-color".to_string());
    }

    if let Some(identity_name) = identity_name {
        args.push("--identity".to_string());
        args.push(identity_name.to_string());
    }

    if let Some(l) = launch_config {
        args.push("--launch-config".to_string());
        args.push(l);
    }

    if let Some(t) = trusted_identities {
        args.push("--trusted-identities".to_string());
        args.push(t.to_string())
    } else if let Some(t) = trusted_identities_file {
        args.push("--trusted-identities-file".to_string());
        args.push(
            t.to_str()
                .unwrap_or_else(|| panic!("unsupported path {t:?}"))
                .to_string(),
        );
    } else if let Some(t) = reload_from_trusted_identities_file {
        args.push("--reload-from-trusted-identities-file".to_string());
        args.push(
            t.to_str()
                .unwrap_or_else(|| panic!("unsupported path {t:?}"))
                .to_string(),
        );
    }

    if let Some(credential) = credential {
        args.push("--credential".to_string());
        args.push(credential.to_string());
    }

    if let Some(trust_context) = trust_context {
        args.push("--trust-context".to_string());
        args.push(trust_context.name());
    }

    if let Some(project_name) = project_name {
        args.push("--project".to_string());
        args.push(project_name.to_string());
    }

    args.push(name.to_owned());

    run_ockam(args).await
}

/// Run the ockam command line with specific arguments
pub async fn run_ockam(args: Vec<String>) -> miette::Result<()> {
    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = get_env_with_default("OCKAM", current_exe().unwrap_or_else(|_| "ockam".into()))
        .into_diagnostic()?;
    Command::new(ockam_exe)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .into_diagnostic()
        .context("failed to spawn node")?;
    Ok(())
}
