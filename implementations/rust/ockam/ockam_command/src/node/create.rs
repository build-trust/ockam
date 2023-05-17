use clap::Args;
use colorful::Colorful;
use rand::prelude::random;
use tokio::time::{sleep, Duration};

use anyhow::{anyhow, Context as _};
use minicbor::{Decoder, Encode};
use std::io::{self, Read};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};
use tracing::error;

use crate::node::util::{add_project_info_to_node_state, init_node_state, spawn_node};
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::util::api::{parse_trust_context, TrustContextConfigBuilder, TrustContextOpts};
use crate::util::node_rpc;
use crate::util::{api, parse_node_name, RpcBuilder};
use crate::util::{bind_to_port_check, embedded_node_that_is_not_stopped, exitcode};
use crate::{
    docs, identity, node::show::print_query_status, util::find_available_port, CommandGlobalOpts,
    Result,
};
use ockam::{Address, AsyncTryClone, TcpListenerOptions};
use ockam::{Context, TcpTransport};
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::authority_node;
use ockam_api::nodes::models::transport::CreateTransportJson;
use ockam_api::nodes::service::{ApiTransport, NodeManagerTrustOptions};
use ockam_api::{
    bootstrapped_identities_store::PreTrustedIdentities,
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{
        service::{
            NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
        },
        NodeManager, NodeManagerWorker, NODEMANAGER_ADDR,
    },
};
use ockam_core::api::{RequestBuilder, Response, Status};
use ockam_core::{route, AllowAll, LOCAL};

use super::show::is_node_up;
use super::util::check_default;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the node (Optional).
    #[arg(hide_default_value = true, default_value_t = hex::encode(& random::< [u8; 4] > ()))]
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

    /// ockam_command started a child process to run this node in foreground.
    #[arg(display_order = 900, long, hide = true)]
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

    #[arg(long = "vault", value_name = "VAULT")]
    vault: Option<String>,

    #[arg(long = "identity", value_name = "IDENTITY")]
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
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            exit_on_eof: false,
            tcp_listener_address: "127.0.0.1:0".to_string(),
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
            trust_context_opts: TrustContextOpts::default(),
        }
    }
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if self.foreground {
            // Create a new node in the foreground (i.e. in this OS process)
            if let Err(e) = create_foreground_node(&options, &self) {
                error!(%e);
                eprintln!("{e:?}");
                std::process::exit(e.code());
            }
        } else {
            // Create a new node running in the background (i.e. another, new OS process)
            node_rpc(run_impl, (options, self))
        }
    }

    fn overwrite_addr(&self) -> Result<Self> {
        let cmd = self.clone();
        let addr: SocketAddr = if &cmd.tcp_listener_address == "127.0.0.1:0" {
            let port = find_available_port().context("failed to acquire available port")?;
            SocketAddr::new(IpAddr::from_str("127.0.0.1")?, port)
        } else {
            cmd.tcp_listener_address.parse()?
        };
        Ok(Self {
            tcp_listener_address: addr.to_string(),
            ..cmd
        })
    }
}

pub fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path).context(anyhow!("Not a valid path"))?;
            Config::read(path)
        }
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node_name = &parse_node_name(&cmd.node_name)?;

    if cmd.child_process {
        return Err(crate::Error::new(
            exitcode::CONFIG,
            anyhow!("Cannot create a background node from background node"),
        ));
    }

    // Spawn node in another, new process
    let cmd = cmd.overwrite_addr()?;
    let addr = SocketAddr::from_str(&cmd.tcp_listener_address)?;

    spawn_background_node(&opts, &cmd, addr).await?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    let is_default = check_default(&opts, node_name);

    // TODO: This is a temporary workaround until we have proper
    // support for controling the output of commands
    if opts.global_args.quiet {
        let _ = is_node_up(&mut rpc, true).await?;
        return Ok(());
    }

    print_query_status(&mut rpc, node_name, true, is_default).await?;

    Ok(())
}

fn create_foreground_node(opts: &CommandGlobalOpts, cmd: &CreateCommand) -> crate::Result<()> {
    let cmd = cmd.overwrite_addr()?;
    embedded_node_that_is_not_stopped(run_foreground_node, (opts.clone(), cmd))?;
    Ok(())
}

async fn run_foreground_node(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let cfg = &opts.config;
    let node_name = parse_node_name(&cmd.node_name)?;

    // TODO: remove this special case once the Orchestrator has migrated to the
    // new ockam authority create command
    if node_name == "authority" && cmd.launch_config.is_some() {
        return start_authority_node(ctx, (opts, cmd)).await;
    };

    // This node was initially created as a foreground node
    // and there is no existing state for it yet.
    if !cmd.child_process && opts.state.nodes.get(&node_name).is_err() {
        init_node_state(
            &opts,
            &node_name,
            cmd.vault.as_deref(),
            cmd.identity.as_deref(),
        )
        .await?;
    }

    add_project_info_to_node_state(&node_name, &opts, cfg, &cmd.trust_context_opts).await?;

    let trust_context_config =
        TrustContextConfigBuilder::new(&opts.state, &cmd.trust_context_opts)?
            .with_authority_identity(cmd.authority_identity.as_ref())
            .with_credential_name(cmd.credential.as_ref())
            .build();

    let tcp = TcpTransport::create(&ctx).await?;
    let bind = &cmd.tcp_listener_address;

    // TODO: This is only listening on loopback address, but should use FlowControls anyways
    let (socket_addr, listener_addr) = tcp.listen(&bind, TcpListenerOptions::insecure()).await?;

    let node_state = opts.state.nodes.get(&node_name)?;
    node_state.set_setup(
        &node_state
            .config()
            .setup_mut()
            .set_verbose(opts.global_args.verbose)
            .add_transport(CreateTransportJson::new(
                TransportType::Tcp,
                TransportMode::Listen,
                bind,
            )?),
    )?;

    let projects = cfg.inner().lookup().projects().collect();
    let pre_trusted_identities = load_pre_trusted_identities(&cmd)?;

    let node_man = NodeManager::create(
        &ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            pre_trusted_identities,
        ),
        NodeManagerProjectsOptions::new(projects),
        NodeManagerTransportOptions::new(
            ApiTransport {
                tt: TransportType::Tcp,
                tm: TransportMode::Listen,
                socket_address: socket_addr,
                worker_address: listener_addr,
                flow_control_id: None, // TODO: Replace with proper value when loopbck TCP listener starts using FlowControls
            },
            tcp.async_try_clone().await?,
        ),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await?;
    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker, AllowAll, AllowAll)
        .await?;

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
            ctx.stop().await?;
            return Err(anyhow!("Failed to start services".to_string()).into());
        }
    }

    // Create a channel for communicating back to the main thread
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);

    // Register a handler for SIGINT, SIGTERM, SIGHUP
    let tx_clone = tx.clone();
    let opts_clone = opts.clone();
    ctrlc::set_handler(move || {
        let _ = tx_clone.blocking_send(());
        let _ = opts_clone
            .terminal
            .write_line(format!("{} Ctrl+C signal received", "!".light_yellow()).as_str());
    })
    .expect("Error setting Ctrl+C handler");

    // Spawn a thread to monitor STDIN for EOF
    if cmd.exit_on_eof {
        let tx_clone = tx.clone();
        let opts_clone = opts.clone();
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut handle = io::stdin().lock();
            handle
                .read_to_end(&mut buffer)
                .expect("Error reading from stdin");
            let _ = tx_clone.blocking_send(());
            let _ = opts_clone
                .terminal
                .write_line(format!("{} EOF received", "!".light_yellow()).as_str());
        });
    }

    // Shutdown on SIGINT, SIGTERM, SIGHUP or EOF
    if rx.recv().await.is_some() {
        opts.state.nodes.get(&node_name)?.kill_process(false)?;
        ctx.stop().await?;
        opts.terminal
            .write_line(format!("{}Node stopped successfully", "✔︎".light_green()).as_str())
            .unwrap();
    }

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

async fn start_services(ctx: &Context, cfg: &Config) -> Result<()> {
    let config = {
        if let Some(sc) = &cfg.startup_services {
            sc.clone()
        } else {
            return Ok(());
        }
    };

    if let Some(cfg) = config.identity {
        if !cfg.disabled {
            println!("starting identity service ...");
            let req = api::start_identity_service(&cfg.address);
            send_req_to_node_manager(ctx, req).await?;
        }
    }
    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let identity = cfg.identity;
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, identity, route![]).await?;
        }
    }
    if let Some(cfg) = config.verifier {
        if !cfg.disabled {
            println!("starting verifier service ...");
            let req = api::start_verifier_service(&cfg.address);
            send_req_to_node_manager(ctx, req).await?;
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

async fn send_req_to_node_manager<T>(ctx: &Context, req: RequestBuilder<'_, T>) -> Result<()>
where
    T: Encode<()>,
{
    let buf: Vec<u8> = ctx
        .send_and_receive(NODEMANAGER_ADDR, req.to_vec()?)
        .await?;
    let mut dec = Decoder::new(&buf);
    let hdr = dec.decode::<Response>()?;
    if hdr.status() != Some(Status::Ok) {
        return Err(anyhow!("Request failed with status: {:?}", hdr.status()).into());
    }
    Ok(())
}

async fn spawn_background_node(
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
    addr: SocketAddr,
) -> crate::Result<()> {
    // Check if the port is used by some other services or process
    if !bind_to_port_check(&addr) {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let node_name = parse_node_name(&cmd.node_name)?;

    // Create node state, including the vault and identity if don't exist
    init_node_state(
        opts,
        &node_name,
        cmd.vault.as_deref(),
        cmd.identity.as_deref(),
    )
    .await?;

    let trust_context_path = match cmd.trust_context_opts.trust_context.clone() {
        Some(tc) => {
            let config = parse_trust_context(&opts.state, &tc)?;
            Some(config.path().unwrap().clone())
        }
        None => None,
    };

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    spawn_node(
        opts,
        opts.global_args.verbose,
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
    )?;

    Ok(())
}

async fn start_authority_node(
    ctx: Context,
    opts: (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let (opts, cmd) = opts;
    let launch_config = cmd
        .launch_config
        .clone()
        .expect("launch config is required for an authority node");
    if let Some(services) = launch_config.startup_services {
        let authenticator_config = services.authenticator.ok_or(crate::Error::new(
            exitcode::CONFIG,
            anyhow!("The authenticator service must be specified for an authority node"),
        ))?;
        let secure_channel_config = services.secure_channel_listener.ok_or(crate::Error::new(
            exitcode::CONFIG,
            anyhow!("The secure channel listener service must be specified for an authority node"),
        ))?;

        // retrieve the authority identity if it has been created before
        // otherwise create a new one
        let identifier = match opts.state.identities.default() {
            Ok(state) => state.config().identifier(),
            Err(_) => {
                let cmd = identity::CreateCommand::new("authority".into(), None);
                cmd.create_identity(opts.clone()).await?
            }
        };

        let trusted_identities = load_pre_trusted_identities(&cmd)
            .map(|ts| ts.unwrap_or(PreTrustedIdentities::Fixed(Default::default())))
            .map_err(|e| crate::Error::new(exitcode::CONFIG, anyhow!("{e}")))?;

        let configuration = authority_node::Configuration {
            identifier,
            storage_path: opts.state.identities.identities_repository_path()?,
            vault_path: opts.state.vaults.default()?.vault_file_path().clone(),
            project_identifier: authenticator_config.project.clone(),
            trust_context_identifier: authenticator_config.project,
            tcp_listener_address: cmd.tcp_listener_address,
            secure_channel_listener_name: Some(secure_channel_config.address),
            authenticator_name: Some(authenticator_config.address),
            trusted_identities,
            no_direct_authentication: true,
            no_token_enrollment: true,
            okta: None,
        };
        authority_node::start_node(&ctx, &configuration).await?;
    }
    Ok(())
}
