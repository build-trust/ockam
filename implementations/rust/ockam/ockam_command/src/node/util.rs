use anyhow::{anyhow, Context as _, Result};
use std::sync::Arc;

use crate::{util::startup, CommandGlobalOpts};
use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::config::cli::OckamConfig as OckamConfigApi;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::{IdentityOverride, NodeManager, NODEMANAGER_ADDR};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};
use tracing::trace;

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::{project, OckamConfig};

pub async fn start_embedded_node(ctx: &Context, cfg: &OckamConfig) -> Result<String> {
    let cmd = CreateCommand::default();

    // Create node directory if it doesn't exist
    tokio::fs::create_dir_all(&cfg.get_node_dir_raw(&cmd.node_name)?).await?;

    // This node was initially created as a foreground node
    if !cmd.child_process {
        create_default_identity_if_needed(ctx, cfg).await?;
    }

    let identity_override = if cmd.skip_defaults {
        None
    } else {
        Some(get_identity_override(ctx, cfg).await?)
    };

    let project_id = match &cmd.project {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let p: ProjectInfo = serde_json::from_str(&s)?;
            let project_id = p.id.as_bytes().to_vec();
            project::config::set_project(cfg, &(&p).into()).await?;
            add_project_authority(p, &cmd.node_name, cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;
    let node_dir = cfg.get_node_dir_raw(&cmd.node_name)?;
    let node_man = NodeManager::create(
        ctx,
        cmd.node_name.clone(),
        node_dir,
        identity_override,
        cmd.skip_defaults || cmd.launch_config.is_some(),
        Some(&cfg.authorities(&cmd.node_name)?.snapshot()),
        project_id,
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;

    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    Ok(cmd.node_name.clone())
}

pub(super) async fn create_default_identity_if_needed(
    ctx: &Context,
    cfg: &OckamConfig,
) -> Result<()> {
    // Get default root vault (create if needed)
    let default_vault_path = cfg.get_default_vault_path().unwrap_or_else(|| {
        let default_vault_path = cli::OckamConfig::directories()
            .config_dir()
            .join("default_vault.json");

        cfg.set_default_vault_path(Some(default_vault_path.clone()));

        default_vault_path
    });

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let vault = Vault::new(Some(Arc::new(storage)));

    // Get default root identity (create if needed)
    if cfg.get_default_identity().is_none() {
        let identity = Identity::create(ctx, &vault).await?;
        let exported_data = identity.export().await?;
        cfg.set_default_identity(Some(exported_data));
    };

    cfg.persist_config_updates()?;

    Ok(())
}

pub(super) async fn get_identity_override(
    ctx: &Context,
    cfg: &OckamConfig,
) -> Result<IdentityOverride> {
    // Get default root vault
    let default_vault_path = cfg
        .get_default_vault_path()
        .context("Default vault was not found")?;

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let vault = Vault::new(Some(Arc::new(storage)));

    // Get default root identity
    let default_identity = cfg
        .get_default_identity()
        .context("Default identity was not found")?;

    // Just to check validity
    Identity::import(ctx, &default_identity, &vault).await?;

    Ok(IdentityOverride {
        identity: default_identity,
        vault_path: default_vault_path,
    })
}

pub(super) async fn add_project_authority(
    p: ProjectInfo<'_>,
    node: &str,
    cfg: &OckamConfig,
) -> Result<()> {
    let m = p
        .authority_access_route
        .map(|a| MultiAddr::try_from(&*a))
        .transpose()?;
    let a = p
        .authority_identity
        .map(|a| hex::decode(a.as_bytes()))
        .transpose()?;
    if let Some((a, m)) = a.zip(m) {
        let v = Vault::default();
        let i = PublicIdentity::import(&a, &v).await?;
        let a = cli::Authority::new(a, m);
        cfg.authorities(node)?
            .add_authority(i.identifier().clone(), a)
    } else {
        Err(anyhow!("missing authority in project info"))
    }
}

pub async fn delete_embedded_node(cfg: &OckamConfig, name: &str) {
    // Try removing the node's directory
    if let Ok(dir) = cfg.get_node_dir_raw(name) {
        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}

pub fn delete_all_nodes(opts: CommandGlobalOpts, force: bool) -> anyhow::Result<()> {
    // Try to delete all nodes found in the config file + their associated processes
    let nn: Vec<String> = {
        let inner = &opts.config.inner();
        inner.nodes.iter().map(|(name, _)| name.clone()).collect()
    };
    for node_name in nn.iter() {
        delete_node(&opts, node_name, force)
    }

    // Try to delete dangling embedded nodes directories
    let dirs = OckamConfigApi::directories();
    let nodes_dir = dirs.data_local_dir();
    if nodes_dir.exists() {
        for entry in nodes_dir.read_dir()? {
            let dir = entry?;
            if !dir.file_type()?.is_dir() {
                continue;
            }
            if let Some(dir_name) = dir.file_name().to_str() {
                if !nn.contains(&dir_name.to_string()) {
                    let _ = std::fs::remove_dir_all(dir.path());
                }
            }
        }
    }

    // If force is enabled
    if force {
        // delete the config and nodes directories
        opts.config.remove()?;
        // and all dangling/orphan ockam processes
        if let Ok(cpid) = get_current_pid() {
            let s = System::new_all();
            for (pid, process) in s.processes() {
                if pid != &cpid && process.name() == "ockam" {
                    process.kill();
                }
            }
        }
    } else if let Err(e) = opts.config.persist_config_updates() {
        eprintln!("Failed to update config file. You might need to run the command with --force to delete all config directories");
        return Err(e);
    }
    Ok(())
}

pub fn delete_node(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) {
    trace!(%node_name, "Deleting node");

    // We ignore the result of killing the node process as it could be not
    // found (after a restart or if the user manually deleted it, for example).
    let _ = delete_node_pid(opts, node_name, sigkill);

    delete_node_config(opts, node_name);
}

fn delete_node_pid(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) -> anyhow::Result<()> {
    trace!(%node_name, "Deleting node pid");
    // Stop the process PID if it has one assigned in the config file
    if let Some(pid) = opts.config.get_node_pid(node_name)? {
        startup::stop(pid, sigkill)?;
        // Give some room for the process to stop
        std::thread::sleep(std::time::Duration::from_millis(100));
        // If it fails to bind, the port is still in use, so we try again to stop the process
        let addr = format!("127.0.0.1:{}", opts.config.get_node_port(node_name));
        if std::net::TcpListener::bind(&addr).is_err() {
            startup::stop(pid, sigkill)?;
        }
    }
    Ok(())
}

fn delete_node_config(opts: &CommandGlobalOpts, node_name: &str) {
    trace!(%node_name, "Deleting node config");

    // Try removing the node's directory.
    // If the directory is not found, we ignore the result and continue.
    let _ = opts
        .config
        .get_node_dir_raw(node_name)
        .map(std::fs::remove_dir_all);

    // Try removing the node's info from the config file.
    opts.config.remove_node(node_name);
}
