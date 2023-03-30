use anyhow::{anyhow, Context as _};

use rand::random;
use std::env::current_exe;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::Command;

use ockam::identity::Identity;
use ockam::{Context, TcpListenerTrustOptions, TcpTransport};
use ockam_api::cli_state;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::service::{
    ApiTransport, NodeManagerGeneralOptions, NodeManagerProjectsOptions,
    NodeManagerTransportOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_core::compat::sync::Arc;
use ockam_core::AllowAll;

use crate::node::CreateCommand;
use crate::util::api::ProjectOpts;
use crate::{CommandGlobalOpts, Result};
use ockam::AsyncTryClone;

pub async fn start_embedded_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    project_opts: Option<&ProjectOpts>,
) -> Result<String> {
    start_embedded_node_with_vault_and_identity(ctx, opts, None, None, project_opts).await
}

pub async fn start_embedded_node_with_vault_and_identity(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    vault: Option<&String>,
    identity: Option<&String>,
    project_opts: Option<&ProjectOpts>,
) -> Result<String> {
    let cmd = CreateCommand::default();

    // This node was initially created as a foreground node
    if !cmd.child_process {
        init_node_state(ctx, opts, &cmd.node_name, vault, identity).await?;
    }

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    // This listener gives exclusive access to our node, make sure this is intended
    // + make sure this tcp address is only reachable from the local loopback and/or intended
    // network
    let (socket_addr, listened_worker_address) =
        tcp.listen(&bind, TcpListenerTrustOptions::new()).await?;

    let (trust_context, project_addr) = match project_opts
        .and_then(|o| o.trust_context(opts.state.projects.default().ok().map(|p| p.path)))
    {
        None => (None, None),
        Some(cfg) => (
            Some(cfg.build(tcp.async_try_clone().await?).await),
            cfg.project_addr(),
        ),
    };

    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            None,
        ),
        NodeManagerProjectsOptions::new(project_addr, trust_context),
        NodeManagerTransportOptions::new(
            ApiTransport {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                socket_address: socket_addr,
                worker_address: listened_worker_address,
            },
            tcp,
        ),
    )
    .await?;

    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(
        NODEMANAGER_ADDR,
        node_manager_worker,
        AllowAll, // FIXME: @ac
        AllowAll, // FIXME: @ac
    )
    .await?;

    Ok(cmd.node_name.clone())
}

pub(crate) async fn init_node_state(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    vault: Option<&String>,
    identity: Option<&String>,
) -> Result<()> {
    // Get vault specified in the argument, or get the default
    let vault_state = if let Some(v) = vault {
        opts.state.vaults.get(v)?
    }
    // Or get the default
    else if let Ok(v) = opts.state.vaults.default() {
        v
    } else {
        let n = hex::encode(random::<[u8; 4]>());
        let c = cli_state::VaultConfig::default();
        opts.state.vaults.create(&n, c).await?
    };

    // Get identity specified in the argument
    let identity_state = if let Some(idt) = identity {
        opts.state.identities.get(idt)?
    }
    // Or get the default
    else if let Ok(idt) = opts.state.identities.default() {
        idt
    } else {
        let vault = vault_state.get().await?;
        let identity_name = hex::encode(random::<[u8; 4]>());
        let identity = Identity::create_ext(
            ctx,
            opts.state.identities.authenticated_storage().await?,
            Arc::new(vault),
        )
        .await?;
        let identity_config = cli_state::IdentityConfig::new(&identity).await;
        opts.state
            .identities
            .create(&identity_name, identity_config)?
    };

    // Create the node with the given vault and identity
    let node_config = cli_state::NodeConfigBuilder::default()
        .vault(vault_state.path)
        .identity(identity_state.path)
        .build(&opts.state)?;
    opts.state.nodes.create(node_name, node_config)?;

    Ok(())
}

pub async fn delete_embedded_node(opts: &CommandGlobalOpts, name: &str) {
    let _ = delete_node(opts, name, false);
}

pub fn delete_node(opts: &CommandGlobalOpts, name: &str, force: bool) -> Result<()> {
    opts.state.nodes.delete(name, force)?;
    Ok(())
}

pub fn delete_all_nodes(opts: CommandGlobalOpts, force: bool) -> Result<()> {
    let nodes_states = opts.state.nodes.list()?;
    let mut deletion_errors = Vec::new();
    for s in nodes_states {
        if let Err(e) = opts.state.nodes.delete(&s.config.name, force) {
            deletion_errors.push((s.config.name.clone(), e));
        }
    }
    if !deletion_errors.is_empty() {
        return Err(anyhow!("errors while deleting nodes: {:?}", deletion_errors).into());
    }
    Ok(())
}

pub fn set_default_node(opts: &CommandGlobalOpts, name: &str) -> anyhow::Result<()> {
    opts.state.nodes.set_default(name)?;
    Ok(())
}

pub fn check_default(opts: &CommandGlobalOpts, name: &str) -> bool {
    if let Ok(default) = opts.state.nodes.default() {
        return default.config.name == name;
    }
    false
}

/// A utility function to spawn a new node into foreground mode
#[allow(clippy::too_many_arguments)]
pub fn spawn_node(
    opts: &CommandGlobalOpts,
    verbose: u8,
    name: &str,
    address: &str,
    project_path: Option<&PathBuf>,
    trust_context: Option<String>,
    project: Option<&String>,
    trusted_identities: Option<&String>,
    trusted_identities_file: Option<&PathBuf>,
    reload_from_trusted_identities_file: Option<&PathBuf>,
    launch_config: Option<String>,
) -> crate::Result<()> {
    let mut args = vec![
        match verbose {
            0 => "-vv".to_string(),
            v => format!("-{}", "v".repeat(v as usize)),
        },
        "--no-color".to_string(),
        "node".to_string(),
        "create".to_string(),
        "--tcp-listener-address".to_string(),
        address.to_string(),
        "--foreground".to_string(),
        "--child-process".to_string(),
    ];

    if let Some(path) = project_path {
        args.push("--project-path".to_string());
        let p = path
            .to_str()
            .unwrap_or_else(|| panic!("unsupported path {path:?}"));
        args.push(p.to_string())
    }
    if let Some(name) = project {
        args.push("--project".to_string());
        args.push(name.to_string())
    }
    if let Some(cfg) = trust_context {
        args.push("--trust-context".to_string());
        args.push(cfg)
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

    /*
    if let Some(authority_identities) = authority_identities {
        for authority in authority_identities.iter() {
            let identity = hex::encode(authority.identity());
            args.push("--authority-identity".to_string());
            args.push(identity);
        }
    }

    if let Some(credential) = credential {
        args.push("--credential".to_string());
        args.push(credential.to_string());
    }
    */

    args.push(name.to_owned());

    run_ockam(opts, name, args)
}

/// Run the ockam command line with specific arguments
pub fn run_ockam(
    opts: &CommandGlobalOpts,
    node_name: &str,
    args: Vec<String>,
) -> crate::Result<()> {
    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = current_exe().unwrap_or_else(|_| "ockam".into());
    let node_state = opts.state.nodes.get(node_name)?;

    let (mlog, elog) = { (node_state.stdout_log(), node_state.stderr_log()) };

    let main_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mlog)
        .context("failed to open log path")?;

    let stderr_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(elog)
        .context("failed to open stderr log path")?;

    let child = Command::new(ockam_exe)
        .args(args)
        .stdout(main_log_file)
        .stderr(stderr_log_file)
        .spawn()?;
    node_state.set_pid(child.id() as i32)?;

    Ok(())
}
