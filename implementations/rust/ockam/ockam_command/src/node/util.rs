use anyhow::{anyhow, Context as _, Result};
use std::path::Path;
use std::sync::Arc;

use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::{IdentityOverride, NodeManager, NODEMANAGER_ADDR};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::OckamConfig;

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

    if let Some(path) = &cmd.project {
        add_project_authority(path, &cmd.node_name, cfg).await?
    }

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;
    let node_dir = cfg.get_node_dir_raw(&cmd.node_name)?;
    let mut node_man = NodeManager::create(
        ctx,
        cmd.node_name.clone(),
        node_dir,
        identity_override,
        cmd.skip_defaults || cmd.launch_config.is_some(),
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;

    node_man
        .configure_authorities(&cfg.authorities(&cmd.node_name)?.snapshot())
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

pub(super) async fn add_project_authority<P>(path: P, node: &str, cfg: &OckamConfig) -> Result<()>
where
    P: AsRef<Path>,
{
    let s = tokio::fs::read_to_string(path.as_ref()).await?;
    let p: ProjectInfo = serde_json::from_str(&s)?;
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
