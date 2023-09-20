use std::env::current_exe;
use std::fs::OpenOptions;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::identity::{Identifier, SecureClient};
use ockam::{Context, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{
    add_project_info_to_node_state, init_node_state, CliState, StateDirTrait,
};
use ockam_api::cloud::{AuthorityNode, Controller, ProjectNode};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
    SupervisedNodeManager,
};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::env::get_env_with_default;
use ockam_multiaddr::MultiAddr;

use crate::node::CreateCommand;
use crate::util::api::TrustContextOpts;
use crate::{CommandGlobalOpts, Result};

pub async fn start_node_manager(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    trust_opts: Option<&TrustContextOpts>,
) -> Result<SupervisedNodeManager> {
    start_node_manager_with_vault_and_identity(ctx, &opts.state, None, None, trust_opts).await
}

pub async fn start_node_manager_with_vault_and_identity(
    ctx: &Context,
    cli_state: &CliState,
    vault: Option<String>,
    identity: Option<String>,
    trust_opts: Option<&TrustContextOpts>,
) -> Result<SupervisedNodeManager> {
    let cmd = CreateCommand::default();

    init_node_state(
        cli_state,
        &cmd.node_name,
        vault.as_deref(),
        identity.as_deref(),
    )
    .await?;

    if let Some(p) = trust_opts {
        add_project_info_to_node_state(&cmd.node_name, cli_state, p.project_path.as_ref()).await?;
    } else {
        add_project_info_to_node_state(
            &cmd.node_name,
            cli_state,
            cmd.trust_context_opts.project_path.as_ref(),
        )
        .await?;
    };

    let trust_context_config = match trust_opts {
        Some(t) => t.to_config(cli_state)?.build(),
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
    let bind = cmd.tcp_listener_address;

    let options = TcpListenerOptions::new();
    let listener = tcp.listen(&bind, options).await?;

    let node_manager = SupervisedNodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            cli_state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            None,
        ),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await?;
    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    Ok(node_manager)
}

pub fn delete_node(opts: &CommandGlobalOpts, name: &str, force: bool) -> miette::Result<()> {
    opts.state.nodes.delete_sigkill(name, force)?;
    Ok(())
}

pub fn delete_all_nodes(opts: &CommandGlobalOpts, force: bool) -> miette::Result<()> {
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
    let ockam_exe = get_env_with_default("OCKAM", current_exe().unwrap_or_else(|_| "ockam".into()))
        .into_diagnostic()?;
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

pub struct LocalNode {
    pub(crate) node_manager: SupervisedNodeManager,
    controller: Arc<Controller>,
    opts: CommandGlobalOpts,
}

impl LocalNode {
    pub async fn make(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        trust_opts: Option<&TrustContextOpts>,
    ) -> miette::Result<LocalNode> {
        let node_manager = start_node_manager(ctx, opts, trust_opts).await?;
        let controller = node_manager
            .make_controller_node_client()
            .await
            .into_diagnostic()?;
        Ok(Self {
            node_manager,
            controller: Arc::new(controller),
            opts: opts.clone(),
        })
    }

    pub async fn make_project_node_client(
        &self,
        project_identifier: &Identifier,
        project_address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<ProjectNode> {
        self.node_manager
            .make_project_node_client(
                project_identifier,
                project_address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub async fn make_authority_node_client(
        &self,
        authority_identifier: &Identifier,
        authority_address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNode> {
        self.node_manager
            .make_authority_node_client(
                authority_identifier,
                authority_address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub async fn make_secure_client(
        &self,
        identifier: &Identifier,
        address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<SecureClient> {
        self.node_manager
            .make_secure_client(
                identifier,
                address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub fn node_name(&self) -> String {
        self.node_manager.node_name()
    }
}

impl Deref for LocalNode {
    type Target = Arc<Controller>;

    fn deref(&self) -> &Self::Target {
        &self.controller
    }
}

impl Drop for LocalNode {
    fn drop(&mut self) {
        let _ = self
            .opts
            .state
            .nodes
            .delete_sigkill(self.node_manager.node_name().as_str(), false);
    }
}
