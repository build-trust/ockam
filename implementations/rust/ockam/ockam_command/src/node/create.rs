use clap::Args;
use rand::prelude::random;

use anyhow::{Context as _, Result};
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::node::util::{
    add_project_authority, create_default_identity_if_needed, get_identity_override,
};
use crate::project::ProjectInfo;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::service::start;
use crate::util::{bind_to_port_check, exitcode};
use crate::{
    help,
    node::show::print_query_status,
    node::HELP_DETAIL,
    project,
    util::{connect_to, embedded_node, find_available_port, startup, OckamConfig},
    CommandGlobalOpts,
};
use ockam::{Address, AsyncTryClone, NodeBuilder, TCP};
use ockam::{Context, TcpTransport};
use ockam_api::{
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

    /// Skip credential checks
    #[arg(long, hide = true)]
    pub enable_credential_checks: bool,

    /// Don't share default identity with this node
    #[arg(long, hide = true)]
    pub no_shared_identity: bool,

    /// ockam_command started a child process to run this node in foreground.
    #[arg(display_order = 900, long, hide = true)]
    pub child_process: bool,

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
            enable_credential_checks: false,
            no_shared_identity: false,
            child_process: false,
            launch_config: None,
            no_watchdog: false,
            project: None,
            config: None,
        }
    }
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let verbose = options.global_args.verbose;
        let cfg = &options.config;
        if self.foreground {
            let cmd = self.overwrite_addr().unwrap();
            let addr = SocketAddr::from_str(&cmd.tcp_listener_address).unwrap();
            // HACK: try to get the current node dir.  If it doesn't
            // exist the user PROBABLY started a non-detached node.
            // Thus we need to create the node dir so that subsequent
            // calls to it don't fail
            if cfg.get_node_dir(&cmd.node_name).is_err() {
                println!("Creating node directory...");
                if let Err(e) = cfg.create_node(&cmd.node_name, addr, verbose) {
                    eprintln!(
                        "failed to update node configuration for '{}': {}",
                        cmd.node_name, e
                    );
                    std::process::exit(exitcode::CANTCREAT);
                }

                // Save the config update
                if let Err(e) = cfg.persist_config_updates() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }

            if let Err(e) = run_background_node(cmd, addr, cfg.clone(), options) {
                eprintln!("Ockam node failed: {:?}", e);
            }
        } else {
            if self.child_process {
                eprintln!("Cannot create a background node from background node");
                std::process::exit(exitcode::CONFIG);
            }

            let cmd = self.overwrite_addr().unwrap();
            let addr = SocketAddr::from_str(&cmd.tcp_listener_address).unwrap();
            embedded_node(
                Self::create_background_node,
                (options.clone(), cmd.clone(), addr),
            )
            .unwrap();
            connect_to(
                addr.port(),
                (cfg.clone(), cmd.node_name, true),
                print_query_status,
            );
            if let Some(config_path) = &self.config {
                crate::node::util::run::CommandsRunner::run_node_init(config_path)
                    .context("Failed to run init commands")
                    .unwrap();
                crate::node::util::run::CommandsRunner::run_node_startup(config_path)
                    .context("Failed to startup commands")
                    .unwrap();
            }
        }
    }

    pub async fn create_background_node(
        ctx: Context,
        (opts, cmd, addr): (CommandGlobalOpts, CreateCommand, SocketAddr),
    ) -> crate::Result<()> {
        let verbose = opts.global_args.verbose;
        let cfg = &opts.config;

        // Check if the port is used by some other services or process
        if !bind_to_port_check(&addr) {
            eprintln!("Another process is listening on the provided port!");
            std::process::exit(exitcode::IOERR);
        }

        // FIXME: not really clear why this is causing issues
        if cfg.port_is_used(addr.port()) {
            eprintln!("Another node is listening on the provided port!");
            std::process::exit(exitcode::IOERR);
        }

        // First we create a new node in the configuration so that
        // we can ask it for the correct log path, as well as
        // making sure the watchdog can do its job later on.
        if let Err(e) = cfg.create_node(&cmd.node_name, addr, verbose) {
            eprintln!(
                "failed to update node configuration for '{}': {}",
                cmd.node_name, e
            );
            std::process::exit(exitcode::CANTCREAT);
        }

        // Save the config update
        if let Err(e) = cfg.persist_config_updates() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }

        create_default_identity_if_needed(&ctx, cfg).await?;

        // Construct the arguments list and re-execute the ockam
        // CLI in foreground mode to start the newly created node
        startup::spawn_node(
            &opts.config,
            verbose,
            cmd.skip_defaults,
            cmd.no_shared_identity,
            cmd.enable_credential_checks,
            &cmd.node_name,
            &cmd.tcp_listener_address,
            cmd.project.as_deref(),
        );

        // Unless this CLI was called from another watchdog we
        // start the watchdog here
        if !cmd.no_watchdog {}

        Ok(())
    }

    pub fn overwrite_addr(&self) -> Result<Self> {
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

fn run_background_node(
    c: CreateCommand,
    addr: SocketAddr,
    cfg: OckamConfig,
    opts: CommandGlobalOpts,
) -> Result<()> {
    let (mut ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();

    executor
        .execute(async move {
            let v = run_background_node_impl(&mut ctx, c, addr, cfg, &opts).await;

            match v {
                Err(e) => {
                    eprintln!("Background node error {:?}", e);
                    std::process::exit(1);
                }
                Ok(v) => v,
            }
        })
        .map_err(anyhow::Error::from)
}

async fn run_background_node_impl(
    ctx: &mut Context,
    c: CreateCommand,
    addr: SocketAddr,
    cfg: OckamConfig,
    opts: &CommandGlobalOpts,
) -> Result<()> {
    // This node was initially created as a foreground node
    if !c.child_process {
        create_default_identity_if_needed(ctx, &cfg).await?;
    }

    let identity_override = if c.skip_defaults || c.no_shared_identity {
        None
    } else {
        Some(get_identity_override(ctx, &cfg).await?)
    };

    let project_id = match &c.project {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let p: ProjectInfo = serde_json::from_str(&s)?;
            let project_id = p.id.as_bytes().to_vec();
            project::config::set_project(&cfg, &(&p).into()).await?;
            add_project_authority(p, &c.node_name, &cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = c.tcp_listener_address;
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&c.node_name)?;
    let projects = cfg.inner().lookup().projects().collect();
    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            c.node_name.clone(),
            node_dir,
            c.skip_defaults || c.launch_config.is_some(),
            c.enable_credential_checks,
            identity_override,
        ),
        NodeManagerProjectsOptions::new(
            Some(&cfg.authorities(&c.node_name)?.snapshot()),
            project_id,
            projects,
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

    if let Some(path) = c.launch_config {
        let node_opts = super::NodeOpts {
            api_node: c.node_name,
        };
        start_services(ctx, &tcp, &path, addr, node_opts, opts).await?
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
            start::start_vault_service(ctx, opts, &node_opts.api_node, &cfg.address).await?
        }
    }
    if let Some(cfg) = config.identity {
        if !cfg.disabled {
            println!("starting identity service ...");
            start::start_identity_service(ctx, opts, &node_opts.api_node, &cfg.address).await?
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
            start::start_verifier_service(ctx, opts, &node_opts.api_node, &cfg.address).await?
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
            )
            .await?
        }
    }

    Ok(())
}
