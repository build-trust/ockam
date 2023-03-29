use anyhow::{anyhow, Context as _};

use ockam_api::config::lookup::ProjectLookup;
use ockam_api::trust_context::CredentialRetriever::FromCredentialIssuer;
use ockam_api::trust_context::{AuthorityInfo, CredentialIssuerInfo, TrustContext};
use rand::random;
use std::env::current_exe;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Command;

use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpListenerTrustOptions, TcpTransport};
use ockam_api::cli_state;

use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::service::{
    ApiTransport, NodeManagerGeneralOptions, NodeManagerProjectsOptions,
    NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_core::compat::sync::Arc;
use ockam_core::AllowAll;
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::util::api::ProjectOpts;
use crate::{project, CommandGlobalOpts, OckamConfig, Result};

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
    let cfg = &opts.config;
    let cmd = CreateCommand::default();

    // This node was initially created as a foreground node
    if !cmd.child_process {
        init_node_state(ctx, opts, &cmd.node_name, vault, identity).await?;
    }

    // If a project path or name is provided
    // add a trust context / authority with it.
    if let Some(p) = project_opts {
        add_project_info_to_node_state(opts, &cmd.node_name, cfg, p).await?
    } else {
        match &cmd.project {
            Some(path) => {
                let s = tokio::fs::read_to_string(path).await?;
                let p: ProjectInfo = serde_json::from_str(&s)?;
                let project_id = p.id.to_string();
                project::config::set_project(cfg, &(&p).into()).await?;
                add_project_authority_from_project_info(p, &cmd.node_name, cfg).await?;
                Some(project_id)
            }
            None => None,
        }
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    // This listener gives exclusive access to our node, make sure this is intended
    // + make sure this tcp address is only reachable from the local loopback and/or intended
    // network
    let (socket_addr, listened_worker_address) =
        tcp.listen(&bind, TcpListenerTrustOptions::new()).await?;

    let projects = cfg.inner().lookup().projects().collect();

    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            None,
        ),
        NodeManagerProjectsOptions::new(projects),
        NodeManagerTransportOptions::new(
            ApiTransport {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                socket_address: socket_addr,
                worker_address: listened_worker_address,
            },
            tcp,
        ),
        NodeManagerTrustOptions::new(Some(cfg.authorities(&cmd.node_name)?.snapshot())),
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

pub async fn add_project_info_to_node_state(
    opts: &CommandGlobalOpts,
    node_name: &str,
    cfg: &OckamConfig,
    project_opts: &ProjectOpts,
) -> Result<Option<String>> {
    let proj_path = if let Some(path) = project_opts.project_path.clone() {
        Some(path)
    } else if let Ok(proj) = opts.state.projects.default() {
        Some(proj.path)
    } else {
        None
    };

    match &proj_path {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let proj_info: ProjectInfo = serde_json::from_str(&s)?;
            let proj_lookup = ProjectLookup::from_project(&(&proj_info).into()).await?;

            // FIXME What is this doing?.  We need to simplify how this work.
            //       we also need to remove project names from routes, as nodes
            //       are started with _one_  project.
            project::config::set_project(cfg, &(&proj_info).into()).await?;

            if let Some(a) = proj_lookup.authority {
                add_project_authority(
                    a.identity().to_vec(),
                    a.address().clone(),
                    node_name,
                    cfg,
                    &proj_lookup.id,
                )
                .await?;
            }
            Ok(Some(proj_lookup.id))
        }
        None => Ok(None),
    }
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

pub(super) async fn add_project_authority(
    authority_identity: Vec<u8>,
    authority_access_route: MultiAddr,
    node: &str,
    cfg: &OckamConfig,
    project_id: &str,
) -> Result<()> {
    let i = PublicIdentity::import(&authority_identity, Vault::create()).await?;
    let identifier = i.identifier().clone();
    let tc = TrustContext::new_with_id(
        AuthorityInfo::new(
            i,
            Some(FromCredentialIssuer(CredentialIssuerInfo::new(
                authority_access_route,
            ))),
        ),
        project_id.to_string(),
    );
    cfg.authorities(node)?.add_authority(identifier, tc)
}

pub(super) async fn add_project_authority_from_project_info(
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
        add_project_authority(a, m, node, cfg, &p.id).await
    } else {
        Err(anyhow!("missing authority in project info").into())
    }
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
    project: Option<&Path>,
    trusted_identities: Option<&String>,
    trusted_identities_file: Option<&PathBuf>,
    reload_from_trusted_identities_file: Option<&PathBuf>,
    launch_config: Option<String>,
    authority: Option<&Vec<TrustContext>>,
    credential: Option<&String>,
    authority_identities: Option<&Vec<String>>,
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

    if let Some(path) = project {
        args.push("--project".to_string());
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

    if let Some(trust_contexts) = authority {
        for authority in trust_contexts.iter() {
            args.push("--authority".to_string());
            args.push(serde_json::to_string(authority)?);
        }
    }

    if let Some(credential) = credential {
        args.push("--credential".to_string());
        args.push(credential.to_string());
    }

    if let Some(identities) = authority_identities {
        for identity in identities.iter() {
            args.push("--authority-identity".to_string());
            args.push(identity.to_string());
        }
    }

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
