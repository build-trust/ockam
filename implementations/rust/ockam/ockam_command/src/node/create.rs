use std::sync::Arc;
use std::{path::PathBuf, process, str::FromStr};

use clap::Args;
use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use minicbor::{Decoder, Encode};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio::try_join;

use ockam::{Address, AsyncTryClone, TcpListenerOptions};
use ockam::{Context, TcpTransport};
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::cli_state::{add_project_info_to_node_state, init_node_state, random_name};
use ockam_api::nodes::models::transport::CreateTransportJson;
use ockam_api::nodes::service::NodeManagerTrustOptions;
use ockam_api::nodes::BackgroundNode;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{
    bootstrapped_identities_store::PreTrustedIdentities,
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{
        service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
        NodeManagerWorker, NODEMANAGER_ADDR,
    },
};
use ockam_core::api::{Request, ResponseHeader, Status};
use ockam_core::{route, LOCAL};

use crate::node::util::{spawn_node, NodeManagerDefaults};
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::terminal::OckamColor;
use crate::util::api::TrustContextOpts;
use crate::util::{api, parse_node_name};
use crate::util::{embedded_node_that_is_not_stopped, exitcode};
use crate::util::{local_cmd, node_rpc};
use crate::{docs, shutdown, CommandGlobalOpts, Result};
use crate::{fmt_log, fmt_ok};

use super::show::is_node_up;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the node.
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub node_name: String,

    /// Run the node in foreground.
    #[arg(display_order = 900, long, short)]
    pub foreground: bool,

    /// Watch stdin for EOF
    #[arg(display_order = 900, long = "exit-on-eof", short)]
    pub exit_on_eof: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// `node create` started a child process to run this node in foreground.
    #[arg(long, hide = true)]
    pub child_process: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(long, hide = true, value_parser = parse_launch_config)]
    pub launch_config: Option<Config>,

    #[arg(long, group = "trusted")]
    pub trusted_identities: Option<String>,
    #[arg(long, group = "trusted")]
    pub trusted_identities_file: Option<PathBuf>,
    #[arg(long, group = "trusted")]
    pub reload_from_trusted_identities_file: Option<PathBuf>,

    /// Name of the Vault that the node will use.
    #[arg(long = "vault", value_name = "VAULT_NAME")]
    vault: Option<String>,

    /// Name of the Identity that the node will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,

    #[arg(long)]
    pub authority_identity: Option<String>,

    #[arg(long = "credential", value_name = "CREDENTIAL_NAME")]
    pub credential: Option<String>,

    #[command(flatten)]
    pub trust_context_opts: TrustContextOpts,
}

impl Default for CreateCommand {
    fn default() -> Self {
        let node_manager_defaults = NodeManagerDefaults::default();
        Self {
            node_name: random_name(),
            exit_on_eof: false,
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            foreground: false,
            child_process: false,
            launch_config: None,
            vault: None,
            identity: None,
            trusted_identities: None,
            trusted_identities_file: None,
            reload_from_trusted_identities_file: None,
            authority_identity: None,
            credential: None,
            trust_context_opts: node_manager_defaults.trust_context_opts,
        }
    }
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if !self.child_process {
            if let Ok(state) = opts.state.nodes.get(&self.node_name) {
                if state.is_running() {
                    eprintln!(
                        "{:?}",
                        miette!("Node {} is already running", self.node_name)
                    );
                    std::process::exit(exitcode::SOFTWARE);
                }
            }
        }
        if self.foreground {
            local_cmd(foreground_mode(opts, self));
        } else {
            node_rpc(background_mode, (opts, self))
        }
    }

    pub fn logging_to_file(&self) -> bool {
        // Background nodes will spawn a foreground node in a child process.
        // In that case, the child process will log to files.
        if self.child_process {
            true
        }
        // The main process will log to stdout only if it's a foreground node.
        else {
            !self.foreground
        }
    }

    pub fn logging_to_stdout(&self) -> bool {
        !self.logging_to_file()
    }
}

pub fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path)
                .into_diagnostic()
                .wrap_err(miette!("Not a valid path"))?;
            Config::read(path)
        }
    }
}

// Create a new node running in the background (i.e. another, new OS process)
pub(crate) async fn background_mode(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let node_name = &parse_node_name(&cmd.node_name)?;
    opts.terminal.write_line(&fmt_log!(
        "Creating Node {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;

    if cmd.child_process {
        return Err(miette!(
            "Cannot create a background node from background node"
        ));
    }

    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        spawn_background_node(&opts, cmd.clone()).await?;
        let mut node = BackgroundNode::create(&ctx, &opts.state, node_name).await?;
        let is_node_up = is_node_up(&ctx, node_name, &mut node, opts.state.clone(), true).await?;
        *is_finished.lock().await = true;
        Ok(is_node_up)
    };

    let output_messages = vec![
        format!("Creating node..."),
        format!("Starting services..."),
        format!("Loading any pre-trusted identities..."),
    ];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (_response, _) = try_join!(send_req, progress_output)?;

    opts.clone()
        .terminal
        .stdout()
        .plain(
            fmt_ok!(
                "Node {} created successfully\n\n",
                node_name
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!("To see more details on this node, run:\n")
                + &fmt_log!(
                    "{}",
                    "ockam node show".color(OckamColor::PrimaryResource.color())
                ),
        )
        .write_line()?;

    Ok(())
}

// Create a new node in the foreground (i.e. in this OS process)
fn foreground_mode(opts: CommandGlobalOpts, cmd: CreateCommand) -> miette::Result<()> {
    embedded_node_that_is_not_stopped(run_foreground_node, (opts, cmd))?;
    Ok(())
}

async fn run_foreground_node(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let node_name = parse_node_name(&cmd.node_name)?;

    // This node was initially created as a foreground node
    // and there is no existing state for it yet.
    if !cmd.child_process && !opts.state.nodes.exists(&node_name) {
        init_node_state(
            &opts.state,
            &node_name,
            cmd.vault.as_deref(),
            cmd.identity.as_deref(),
        )
        .await?;
    }

    add_project_info_to_node_state(
        &node_name,
        &opts.state,
        cmd.trust_context_opts.project_path.as_ref(),
    )
    .await?;

    let trust_context_config = cmd
        .trust_context_opts
        .to_config(&opts.state)?
        .with_authority_identity(cmd.authority_identity.as_ref())
        .with_credential_name(cmd.credential.as_ref())
        .build();

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&cmd.tcp_listener_address, options)
        .await
        .into_diagnostic()?;

    let node_state = opts.state.nodes.get(&node_name)?;
    node_state.set_pid(process::id() as i32)?;
    node_state.set_setup(
        &node_state
            .config()
            .setup_mut()
            .set_verbose(opts.global_args.verbose)
            .set_api_transport(
                CreateTransportJson::new(
                    TransportType::Tcp,
                    TransportMode::Listen,
                    &listener.socket_address().to_string(),
                )
                .into_diagnostic()?,
            ),
    )?;

    let pre_trusted_identities = load_pre_trusted_identities(&cmd)?;

    let node_man = InMemoryNode::new(
        &ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            pre_trusted_identities,
            cmd.launch_config.is_none(),
            true,
        ),
        NodeManagerTransportOptions::new(
            listener.flow_control_id().clone(),
            tcp.async_try_clone().await.into_diagnostic()?,
        ),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;
    let node_manager_worker = NodeManagerWorker::new(Arc::new(node_man));

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await
        .into_diagnostic()?;

    if let Some(config) = &cmd.launch_config {
        if start_services(&ctx, config).await.is_err() {
            //TODO: Process should terminate on any error during its setup phase,
            //      not just during the start_services.
            //TODO: This sleep here is a workaround on some orchestrated environment,
            //      the lmdb db, that is used for policy storage, fails to be re-opened
            //      if it's still opened from another docker container, where they share
            //      the same pid. By sleeping for a while we let this container be promoted
            //      and the other being terminated, so when restarted it works.  This is
            //      FAR from ideal.
            sleep(Duration::from_secs(10)).await;
            ctx.stop().await.into_diagnostic()?;
            return Err(miette!("Failed to start services"));
        }
    }

    // Create a channel for communicating back to the main thread
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);
    shutdown::wait(
        opts.terminal.clone(),
        cmd.exit_on_eof,
        opts.global_args.quiet,
        tx,
        &mut rx,
    )
    .await?;

    // Try to stop node; it might have already been stopped or deleted (e.g. when running `node delete --all`)
    if let Ok(state) = opts.state.nodes.get(&node_name) {
        let _ = state.kill_process(false);
    }
    ctx.stop().await.into_diagnostic()?;
    opts.terminal
        .write_line(format!("{}Node stopped successfully", "✔︎".light_green()).as_str())
        .unwrap();

    Ok(())
}

pub fn load_pre_trusted_identities(cmd: &CreateCommand) -> Result<Option<PreTrustedIdentities>> {
    let command = cmd.clone();
    let pre_trusted_identities = match (
        command.trusted_identities,
        command.trusted_identities_file,
        command.reload_from_trusted_identities_file,
    ) {
        (Some(val), _, _) => Some(PreTrustedIdentities::new_from_string(&val)?),
        (_, Some(val), _) => Some(PreTrustedIdentities::new_from_disk(val, false)?),
        (_, _, Some(val)) => Some(PreTrustedIdentities::new_from_disk(val, true)?),
        _ => None,
    };
    Ok(pre_trusted_identities)
}

async fn start_services(ctx: &Context, cfg: &Config) -> miette::Result<()> {
    let config = {
        if let Some(sc) = &cfg.startup_services {
            sc.clone()
        } else {
            return Ok(());
        }
    };

    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let identity = cfg.identity;
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, identity, route![]).await?;
        }
    }
    if let Some(cfg) = config.authenticator {
        if !cfg.disabled {
            println!("starting authenticator service ...");
            let req = api::start_authenticator_service(&cfg.address, &cfg.project);
            send_req_to_node_manager(ctx, req).await?;
        }
    }
    if let Some(cfg) = config.okta_identity_provider {
        if !cfg.disabled {
            println!("starting okta identity provider service ...");
            let req = api::start_okta_service(&cfg);
            send_req_to_node_manager(ctx, req).await?;
        }
    }

    Ok(())
}

async fn send_req_to_node_manager<T>(ctx: &Context, req: Request<T>) -> Result<()>
where
    T: Encode<()>,
{
    let buf: Vec<u8> = ctx
        .send_and_receive(NODEMANAGER_ADDR, req.to_vec()?)
        .await?;
    let mut dec = Decoder::new(&buf);
    let hdr = dec.decode::<ResponseHeader>()?;
    if hdr.status() != Some(Status::Ok) {
        return Err(miette!("Request failed with status: {:?}", hdr.status()).into());
    }
    Ok(())
}

pub async fn spawn_background_node(
    opts: &CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let node_name = parse_node_name(&cmd.node_name)?;
    // Create node state, including the vault and identity if don't exist
    init_node_state(
        &opts.state,
        &node_name,
        cmd.vault.as_deref(),
        cmd.identity.as_deref(),
    )
    .await?;

    let trust_context_path = match cmd.trust_context_opts.trust_context.clone() {
        Some(tc) => {
            let config = opts.state.trust_contexts.read_config_from_path(&tc)?;
            Some(config.path().unwrap().clone())
        }
        None => None,
    };

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    spawn_node(
        opts,
        &node_name,
        &cmd.tcp_listener_address,
        cmd.trust_context_opts.project_path.as_ref(),
        cmd.trusted_identities.as_ref(),
        cmd.trusted_identities_file.as_ref(),
        cmd.reload_from_trusted_identities_file.as_ref(),
        cmd.launch_config
            .as_ref()
            .map(|config| serde_json::to_string(config).unwrap()),
        cmd.authority_identity.as_ref(),
        cmd.credential.as_ref(),
        trust_context_path.as_ref(),
        cmd.trust_context_opts.project.as_ref(),
        cmd.logging_to_file(),
    )?;

    Ok(())
}
