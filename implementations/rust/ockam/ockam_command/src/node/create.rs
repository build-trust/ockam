use clap::Args;
use rand::prelude::random;

use anyhow::{anyhow, Context as _, Result};
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::error;

use crate::project::ProjectInfo;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::service::start;
use crate::util::{bind_to_port_check, embedded_node_that_is_not_stopped, exitcode};
use crate::{
    help,
    node::show::print_query_status,
    node::HELP_DETAIL,
    project,
    util::{find_available_port, startup},
    CommandGlobalOpts,
};
use crate::{node::util::run::CommandsRunner, util::node_rpc};
use crate::{
    node::util::{add_project_authority, create_default_identity_if_needed, get_identity_override},
    util::RpcBuilder,
};
use ockam::{Address, AsyncTryClone, TCP};
use ockam::{Context, TcpTransport};
use ockam_api::{
    authenticator::direct::types::OneTimeCode,
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{
        service::{
            NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
        },
        NodeManager, NodeManagerWorker, NODEMANAGER_ADDR,
    },
};
use ockam_core::LOCAL;

/// Create Nodes
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Name of the node (Optional).
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    pub node_name: String,

    /// Run the node in foreground.
    #[arg(display_order = 900, long, short)]
    pub foreground: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// Skip creation of default Vault and Identity
    #[arg(long, short, hide = true)]
    pub skip_defaults: bool,

    #[arg(long, hide = true)]
    pub no_shared_identity: bool,

    /// ockam_command started a child process to run this node in foreground.
    #[arg(display_order = 900, long, hide = true)]
    pub child_process: bool,

    /// An enrollment token to allow this node to enroll into a project.
    #[arg(long = "enrollment-token", value_name = "ENROLLMENT_TOKEN", value_parser = otc_parser)]
    token: Option<OneTimeCode>,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(long, hide = true)]
    pub launch_config: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub no_watchdog: bool,

    #[arg(long, hide = true)]
    pub project: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub config: Option<PathBuf>,
}

impl Default for CreateCommand {
    fn default() -> Self {
        Self {
            node_name: hex::encode(&random::<[u8; 4]>()),
            foreground: false,
            tcp_listener_address: "127.0.0.1:0".to_string(),
            skip_defaults: false,
            no_shared_identity: false,
            child_process: false,
            launch_config: None,
            no_watchdog: false,
            project: None,
            config: None,
            token: None,
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

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node_name = &cmd.node_name;
    let cfg = &opts.config;
    if cmd.child_process {
        return Err(crate::Error::new(
            exitcode::CONFIG,
            anyhow!("Cannot create a background node from background node"),
        ));
    }

    // Spawn node in another, new process
    let cmd = cmd.overwrite_addr()?;
    let addr = SocketAddr::from_str(&cmd.tcp_listener_address)?;
    spawn_background_node(&ctx, &opts, &cmd, addr).await?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    let port = cfg.get_node_port(node_name)?;
    print_query_status(&mut rpc, port, node_name, true).await?;

    // Run init and startup commands
    if let Some(config_path) = &cmd.config {
        let node_config = cfg.node(&cmd.node_name)?;
        let commands = CommandsRunner::new(config_path.clone())?;
        node_config.commands().set(commands.commands)?;
        CommandsRunner::run_node_init(config_path).context("Failed to run init commands")?;
        CommandsRunner::run_node_startup(config_path).context("Failed to startup commands")?;
    }
    Ok(())
}

fn create_foreground_node(opts: &CommandGlobalOpts, cmd: &CreateCommand) -> crate::Result<()> {
    let verbose = opts.global_args.verbose;
    let node_name = &cmd.node_name;
    let cfg = &opts.config;

    let cmd = cmd.overwrite_addr()?;
    let addr = SocketAddr::from_str(&cmd.tcp_listener_address)?;

    // HACK: try to get the current node dir.  If it doesn't
    // exist the user PROBABLY started a non-detached node.
    // Thus we need to create the node dir so that subsequent
    // calls to it don't fail
    if cfg.get_node_dir(node_name).is_err() {
        println!("Creating node directory...");
        cfg.create_node(node_name, addr, verbose)?;
        cfg.persist_config_updates()?;
    }

    embedded_node_that_is_not_stopped(run_foreground_node, (opts.clone(), cmd, addr))?;
    Ok(())
}

async fn run_foreground_node(
    ctx: Context,
    (opts, cmd, addr): (CommandGlobalOpts, CreateCommand, SocketAddr),
) -> crate::Result<()> {
    let cfg = &opts.config;

    // This node was initially created as a foreground node
    if !cmd.child_process {
        create_default_identity_if_needed(&ctx, cfg).await?;
    }

    let identity_override = if cmd.skip_defaults || cmd.no_shared_identity {
        None
    } else {
        Some(get_identity_override(&ctx, cfg).await?)
    };

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

    let tcp = TcpTransport::create(&ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&cmd.node_name)?;
    let projects = cfg.inner().lookup().projects().collect();
    let node_man = NodeManager::create(
        &ctx,
        NodeManagerGeneralOptions::new(
            cmd.node_name.clone(),
            node_dir,
            cmd.skip_defaults || cmd.launch_config.is_some(),
            identity_override,
        ),
        NodeManagerProjectsOptions::new(
            Some(&cfg.authorities(&cmd.node_name)?.snapshot()),
            project_id,
            projects,
            cmd.token,
        ),
        NodeManagerTransportOptions::new(
            (TransportType::Tcp, TransportMode::Listen, bind),
            tcp.async_try_clone().await?,
        ),
    )
    .await?;
    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await?;

    if let Some(path) = &cmd.launch_config {
        let node_opts = super::NodeOpts {
            api_node: cmd.node_name,
        };
        start_services(&ctx, &tcp, path, addr, node_opts, &opts).await?
    }

    Ok(())
}

async fn start_services(
    ctx: &Context,
    tcp: &TcpTransport,
    cfg: &Path,
    addr: SocketAddr,
    node_opts: super::NodeOpts,
    opts: &CommandGlobalOpts,
) -> Result<()> {
    let config = {
        let c = Config::read(cfg)?;
        if let Some(sc) = c.startup_services {
            sc
        } else {
            return Ok(());
        }
    };

    let addr = Address::from((TCP, addr.to_string()));
    tcp.connect(addr.address()).await?;

    if let Some(cfg) = config.vault {
        if !cfg.disabled {
            println!("starting vault service ...");
            start::start_vault_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.identity {
        if !cfg.disabled {
            println!("starting identity service ...");
            start::start_identity_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let rte = addr.clone().into();
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, rte).await?;
        }
    }
    if let Some(cfg) = config.verifier {
        if !cfg.disabled {
            println!("starting verifier service ...");
            start::start_verifier_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.authenticator {
        if !cfg.disabled {
            println!("starting authenticator service ...");
            start::start_authenticator_service(
                ctx,
                opts,
                &node_opts.api_node,
                &cfg.address,
                &cfg.enrollers,
                &cfg.project,
                Some(tcp),
            )
            .await?
        }
    }
    if let Some(cfg) = config.okta_identity_provider {
        if !cfg.disabled {
            println!("starting okta identity provider service ...");
            start::start_okta_identity_provider(ctx, opts, &node_opts.api_node, &cfg, Some(tcp))
                .await?
        }
    }

    Ok(())
}

async fn spawn_background_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
    addr: SocketAddr,
) -> crate::Result<()> {
    let verbose = opts.global_args.verbose;
    let cfg = &opts.config;

    // Check if the port is used by some other services or process
    if !bind_to_port_check(&addr) || cfg.port_is_used(addr.port()) {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    // First we create a new node in the configuration so that
    // we can ask it for the correct log path, as well as
    // making sure the watchdog can do its job later on.
    cfg.create_node(&cmd.node_name, addr, verbose)?;
    cfg.persist_config_updates()?;

    create_default_identity_if_needed(ctx, cfg).await?;

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    startup::spawn_node(
        &opts.config,
        verbose,
        cmd.skip_defaults,
        cmd.no_shared_identity,
        &cmd.node_name,
        &cmd.tcp_listener_address,
        cmd.project.as_deref(),
        cmd.token.as_ref(),
    )?;

    Ok(())
}

fn otc_parser(val: &str) -> anyhow::Result<OneTimeCode> {
    let bytes = hex::decode(val)?;
    let code = <[u8; 32]>::try_from(bytes.as_slice())?;
    Ok(code.into())
}
