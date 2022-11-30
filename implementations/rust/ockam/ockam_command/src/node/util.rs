use anyhow::{anyhow, Result};
use rand::random;
use tracing::trace;

use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpTransport};
use ockam_api::cli_state;
use ockam_api::config::cli;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::CommandGlobalOpts;
use crate::{project, OckamConfig};

pub async fn start_embedded_node(ctx: &Context, opts: &CommandGlobalOpts) -> Result<String> {
    start_embedded_node_with_vault_and_identity(ctx, opts, None, None).await
}

pub async fn start_embedded_node_with_vault_and_identity(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    vault: Option<&String>,
    identity: Option<&String>,
) -> Result<String> {
    let cfg = &opts.config;
    let cmd = CreateCommand::default();

    // This node was initially created as a foreground node
    if !cmd.child_process {
        init_node_state(ctx, opts, &cmd.node_name, vault, identity).await?;
    }

    let project_id = match &cmd.project {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let p: ProjectInfo = serde_json::from_str(&s)?;
            let project_id = p.id.to_string();
            project::config::set_project(cfg, &(&p).into()).await?;
            add_project_authority(p, &cmd.node_name, cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;

    let projects = cfg.inner().lookup().projects().collect();
    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            cmd.node_name.clone(),
            cmd.skip_defaults || cmd.launch_config.is_some(),
        ),
        NodeManagerProjectsOptions::new(
            Some(&cfg.authorities(&cmd.node_name)?.snapshot()),
            project_id,
            projects,
            None,
        ),
        NodeManagerTransportOptions::new((TransportType::Tcp, TransportMode::Listen, bind), tcp),
    )
    .await?;

    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await?;

    Ok(cmd.node_name.clone())
}

pub(super) async fn init_node_state(
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
        let c = cli_state::VaultConfig::fs_default(&n)?;
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
        let vault = vault_state.config.get().await?;
        let identity_name = hex::encode(random::<[u8; 4]>());
        let identity = Identity::create(ctx, &vault).await?;
        let identity_config = cli_state::IdentityConfig::new(&identity).await;
        opts.state
            .identities
            .create(&identity_name, identity_config)?
    };

    // Create the node with the given vault and identity
    let node_config = cli_state::NodeConfigBuilder::default()
        .vault(vault_state.path)
        .identity(identity_state.path)
        .build()?;
    opts.state.nodes.create(node_name, node_config)?;

    Ok(())
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

pub async fn delete_embedded_node(opts: &CommandGlobalOpts, name: &str) {
    delete_node(opts, name, false)
}

pub fn delete_node(opts: &CommandGlobalOpts, name: &str, force: bool) {
    if let Ok(s) = opts.state.nodes.get(name) {
        trace!(%name, "Deleting node");
        let _ = s.delete(force);
    }
}

pub fn delete_all_nodes(opts: CommandGlobalOpts, force: bool) -> anyhow::Result<()> {
    let nodes_states = opts.state.nodes.list()?;
    for s in nodes_states {
        let _ = s.delete(force);
    }
    Ok(())
}
