use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::{Context, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state;
use ockam_api::cli_state::traits::StateItemTrait;
use ockam_api::config::lookup::ProjectLookup;

use ockam_api::cli_state::{ProjectConfig, StateDirTrait};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
    NodeManagerTrustOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use std::env::current_exe;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tracing::{debug, info};

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::util::api::{TrustContextConfigBuilder, TrustContextOpts};
use crate::{CommandGlobalOpts, Result};

pub async fn start_embedded_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    trust_opts: Option<&TrustContextOpts>,
) -> Result<String> {
    start_embedded_node_with_vault_and_identity(ctx, opts, None, None, trust_opts).await
}

pub async fn start_embedded_node_with_vault_and_identity(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    vault: Option<String>,
    identity: Option<String>,
    trust_opts: Option<&TrustContextOpts>,
) -> Result<String> {
    let cmd = CreateCommand::default();

    // This node was initially created as a foreground node
    if !cmd.child_process {
        init_node_state(opts, &cmd.node_name, vault.as_deref(), identity.as_deref()).await?;
    }

    if let Some(p) = trust_opts {
        add_project_info_to_node_state(&cmd.node_name, opts, p).await?;
    } else {
        add_project_info_to_node_state(&cmd.node_name, opts, &cmd.trust_context_opts).await?;
    };

    let trust_context_config = match trust_opts {
        Some(t) => TrustContextConfigBuilder::new(&opts.state, t)?.build(),
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
    let bind = cmd.tcp_listener_address;

    let options = TcpListenerOptions::new();
    let listener = tcp.listen(&bind, options).await?;

    let projects = ProjectLookup::from_state(opts.state.projects.list()?).await?;

    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            None,
        ),
        NodeManagerProjectsOptions::new(projects),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await?;

    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());

    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await?;

    Ok(cmd.node_name.clone())
}

pub async fn add_project_info_to_node_state(
    node_name: &str,
    opts: &CommandGlobalOpts,
    project_opts: &TrustContextOpts,
) -> Result<Option<String>> {
    let proj_path = if let Some(path) = project_opts.project_path.clone() {
        Some(path)
    } else if let Ok(proj) = opts.state.projects.default() {
        Some(proj.path().clone())
    } else {
        None
    };

    match &proj_path {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let proj_info: ProjectInfo = serde_json::from_str(&s)?;
            let proj_lookup = ProjectLookup::from_project(&(&proj_info).into()).await?;
            let proj_config = ProjectConfig::from(&proj_info);
            let state = opts.state.nodes.get(node_name)?;
            state.set_setup(state.config().setup_mut().set_project(proj_lookup.clone()))?;
            opts.state
                .projects
                .overwrite(proj_lookup.name, proj_config)?;
            Ok(Some(proj_lookup.id))
        }
        None => Ok(None),
    }
}

pub(crate) async fn init_node_state(
    opts: &CommandGlobalOpts,
    node_name: &str,
    vault_name: Option<&str>,
    identity_name: Option<&str>,
) -> miette::Result<()> {
    debug!(name=%node_name, "initializing node state");
    // Get vault specified in the argument, or get the default
    let vault_state = opts.state.create_vault_state(vault_name).await?;

    // create an identity for the node
    let identity = opts
        .state
        .get_identities(vault_state.get().await?)
        .await?
        .identities_creation()
        .create_identity()
        .await
        .into_diagnostic()
        .wrap_err("Failed to create identity")?;

    let identity_state = opts
        .state
        .create_identity_state(&identity.identifier(), identity_name)
        .await?;

    // Create the node with the given vault and identity
    let node_config = cli_state::NodeConfigBuilder::default()
        .vault(vault_state.path().clone())
        .identity(identity_state.path().clone())
        .build(&opts.state)?;
    opts.state.nodes.overwrite(node_name, node_config)?;

    info!(name=%node_name, "node state initialized");
    Ok(())
}

pub async fn delete_embedded_node(opts: &CommandGlobalOpts, name: &str) {
    let _ = delete_node(opts, name, false);
}

pub fn delete_node(opts: &CommandGlobalOpts, name: &str, force: bool) -> miette::Result<()> {
    opts.state.nodes.delete_sigkill(name, force)?;
    Ok(())
}

pub fn delete_all_nodes(opts: CommandGlobalOpts, force: bool) -> miette::Result<()> {
    let nodes_states = opts.state.nodes.list()?;
    let mut deletion_errors = Vec::new();
    for s in nodes_states {
        if let Err(e) = opts.state.nodes.delete_sigkill(s.name(), force) {
            deletion_errors.push((s.name().to_string(), e));
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

pub fn check_default(opts: &CommandGlobalOpts, name: &str) -> bool {
    if let Ok(default) = opts.state.nodes.default() {
        return default.name() == name;
    }
    false
}

/// A utility function to spawn a new node into foreground mode
#[allow(clippy::too_many_arguments)]
pub fn spawn_node(
    opts: &CommandGlobalOpts,
    name: &str,
    address: &str,
    project: Option<&PathBuf>,
    trusted_identities: Option<&String>,
    trusted_identities_file: Option<&PathBuf>,
    reload_from_trusted_identities_file: Option<&PathBuf>,
    launch_config: Option<String>,
    authority_identity: Option<&String>,
    credential: Option<&String>,
    trust_context: Option<&PathBuf>,
    project_name: Option<&String>,
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

    if let Some(path) = project {
        args.push("--project-path".to_string());
        let p = path
            .to_str()
            .unwrap_or_else(|| panic!("unsupported path {path:?}"));
        args.push(p.to_string())
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

    if let Some(ai) = authority_identity {
        args.push("--authority-identity".to_string());
        args.push(ai.to_string());
    }

    if let Some(credential) = credential {
        args.push("--credential".to_string());
        args.push(credential.to_string());
    }

    if let Some(trust_context) = trust_context {
        args.push("--trust-context".to_string());
        args.push(
            trust_context
                .to_str()
                .unwrap_or_else(|| panic!("unsupported path {trust_context:?}"))
                .to_string(),
        );
    }

    if let Some(project_name) = project_name {
        args.push("--project".to_string());
        args.push(project_name.to_string());
    }

    args.push(name.to_owned());

    run_ockam(opts, name, args, logging_to_file)
}

/// Run the ockam command line with specific arguments
pub fn run_ockam(
    opts: &CommandGlobalOpts,
    node_name: &str,
    args: Vec<String>,
    logging_to_file: bool,
) -> miette::Result<()> {
    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = current_exe().unwrap_or_else(|_| "ockam".into());
    let node_state = opts.state.nodes.get(node_name)?;

    let mut cmd = Command::new(ockam_exe);

    if logging_to_file {
        let (mlog, elog) = { (node_state.stdout_log(), node_state.stderr_log()) };
        let main_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(mlog)
            .into_diagnostic()
            .context("failed to open log path")?;
        let stderr_log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(elog)
            .into_diagnostic()
            .context("failed to open stderr log path")?;
        cmd.stdout(main_log_file).stderr(stderr_log_file);
    }

    let child = cmd
        .args(args)
        .stdin(Stdio::null())
        .spawn()
        .into_diagnostic()
        .context("failed to spawn node")?;

    node_state.set_pid(child.id() as i32)?;

    Ok(())
}
