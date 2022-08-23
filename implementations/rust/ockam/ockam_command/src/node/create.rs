use clap::Args;
use rand::prelude::random;

use anyhow::{Context as _, Result};
use std::sync::Arc;
use std::{
    env::current_exe,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::secure_channel_listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::service::start::{self, StartCommand, StartSubCommand};
use crate::util::{bind_to_port_check, exitcode};
use crate::{
    node::show::query_status,
    util::{
        connect_to, embedded_node, find_available_port, startup, ComposableSnippet, OckamConfig,
        Operation,
    },
    CommandGlobalOpts, HELP_TEMPLATE,
};
use ockam::identity::Identity;
use ockam::{Address, AsyncTryClone, TCP};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::nodes::IdentityOverride;
use ockam_api::{
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{NodeManager, NODEMANAGER_ADDR},
};
use ockam_core::LOCAL;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;

const EXAMPLES: &str = "\
EXAMPLES

    # Create a node, with a generated name
    $ ockam node create

    # Create a node, with a specified name - n1
    $ ockam node create n1

    # Create a node, with a specified tcp listener address
    $ ockam node create n1 --tcp-listener-address 127.0.0.1:6001

    # Create a node, and run it in the foreground with verbose traces
    $ ockam node create n1 --foreground -vvv

LEARN MORE
";

#[derive(Clone, Debug, Args)]
/// Create a node.
#[clap(help_template = const_str::replace!(HELP_TEMPLATE, "LEARN MORE", EXAMPLES))]
pub struct CreateCommand {
    /// Name of the node (Optional).
    #[clap(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    pub node_name: String,

    /// Run the node in foreground.
    #[clap(display_order = 900, long, short)]
    pub foreground: bool,

    /// TCP listener address
    #[clap(
        display_order = 900,
        long,
        short,
        name = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// Skip creation of default Vault and Identity
    #[clap(long, short, hide = true)]
    pub skip_defaults: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[clap(long, hide = true)]
    pub launch_config: Option<PathBuf>,

    #[clap(long, hide = true)]
    pub no_watchdog: bool,
}

impl From<&'_ CreateCommand> for ComposableSnippet {
    fn from(cc: &'_ CreateCommand) -> Self {
        Self {
            id: "_start".into(),
            op: Operation::Node {
                api_addr: cc.tcp_listener_address.clone(),
                node_name: cc.node_name.clone(),
            },
            params: vec![],
        }
    }
}

impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: CreateCommand) {
        let verbose = opts.global_args.verbose;
        let cfg = &opts.config;
        if cmd.foreground {
            let cmd = cmd.overwrite_addr().unwrap();
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
                if let Err(e) = cfg.atomic_update().run() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }

            if let Err(e) = embedded_node(setup, (cmd, addr, cfg.clone())) {
                eprintln!("Ockam node failed: {:?}", e,);
            }
        } else {
            let cmd = cmd.overwrite_addr().unwrap();
            let addr = SocketAddr::from_str(&cmd.tcp_listener_address).unwrap();

            Self::create_background_node(&opts, &cmd, &addr).unwrap();
            connect_to(addr.port(), (cfg.clone(), cmd.node_name), query_status);
        }
    }

    pub fn create_background_node(
        opts: &CommandGlobalOpts,
        cmd: &CreateCommand,
        addr: &SocketAddr,
    ) -> Result<()> {
        let verbose = opts.global_args.verbose;
        let cfg = &opts.config;

        // On systems with non-obvious path setups (or during
        // development) re-executing the current binary is a more
        // deterministic way of starting a node.
        let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

        // Check if the port is used by some other services or process
        if !bind_to_port_check(addr) {
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
        if let Err(e) = cfg.create_node(&cmd.node_name, *addr, verbose) {
            eprintln!(
                "failed to update node configuration for '{}': {}",
                cmd.node_name, e
            );
            std::process::exit(exitcode::CANTCREAT);
        }

        // Construct the arguments list and re-execute the ockam
        // CLI in foreground mode to start the newly created node
        startup::spawn_node(
            &ockam,
            &opts.config,
            verbose,
            cmd.skip_defaults,
            &cmd.node_name,
            &cmd.tcp_listener_address,
        );

        let composite = cmd.into();
        let startup_cfg = cfg.get_startup_cfg(&cmd.node_name).unwrap();
        startup_cfg.writelock_inner().commands = vec![composite].into();

        // Save the config update
        if let Err(e) = startup_cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }

        // Unless this CLI was called from another watchdog we
        // start the watchdog here
        if !cmd.no_watchdog {}

        // Save the config update
        if let Err(e) = cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }

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

async fn create_identity_override(ctx: &Context, cfg: &OckamConfig) -> Result<IdentityOverride> {
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
    let identity = match cfg.get_default_identity() {
        None => {
            let identity = Identity::create(ctx, &vault).await?;
            let exported_data = identity.export().await?;
            cfg.set_default_identity(Some(exported_data));

            identity
        }
        Some(identity) => {
            // Just to check validity
            Identity::import(ctx, &identity, &vault).await?
        }
    };

    cfg.atomic_update().run()?;

    let identity_override = IdentityOverride {
        identity: identity.export().await?,
        vault_path: default_vault_path,
    };

    Ok(identity_override)
}

async fn setup(
    mut ctx: Context,
    (c, addr, cfg): (CreateCommand, SocketAddr, OckamConfig),
) -> Result<()> {
    let identity_override = if c.skip_defaults {
        None
    } else {
        Some(create_identity_override(&ctx, &cfg).await?)
    };

    let tcp = TcpTransport::create(&ctx).await?;
    let bind = c.tcp_listener_address;
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&c.node_name).unwrap(); // can't fail because we already checked it
    let node_man = NodeManager::create(
        &ctx,
        c.node_name.clone(),
        node_dir,
        identity_override,
        c.skip_defaults || c.launch_config.is_some(),
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp.async_try_clone().await?,
    )
    .await?;

    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    if let Some(path) = c.launch_config {
        let node_opts = super::NodeOpts {
            api_node: c.node_name,
        };
        start_services(&mut ctx, &tcp, &path, addr, node_opts).await?
    }

    Ok(())
}

async fn start_services(
    ctx: &mut Context,
    tcp: &TcpTransport,
    cfg: &Path,
    addr: SocketAddr,
    node_opts: super::NodeOpts,
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
            let cmd = StartCommand {
                node_opts: node_opts.clone(),
                create_subcommand: StartSubCommand::Vault { addr: cfg.address },
            };
            println!("starting vault service ...");
            start::start_vault_service(ctx, cmd, addr.clone().into()).await?
        }
    }
    if let Some(cfg) = config.identity {
        if !cfg.disabled {
            let cmd = StartCommand {
                node_opts: node_opts.clone(),
                create_subcommand: StartSubCommand::Identity { addr: cfg.address },
            };
            println!("starting identity service ...");
            start::start_identity_service(ctx, cmd, addr.clone().into()).await?
        }
    }
    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers.into();
            let rte = addr.clone().into();
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, rte).await?;
        }
    }
    if let Some(cfg) = config.verifier {
        if !cfg.disabled {
            let cmd = StartCommand {
                node_opts: node_opts.clone(),
                create_subcommand: StartSubCommand::Verifier { addr: cfg.address },
            };
            println!("starting verifier service ...");
            start::start_verifier_service(ctx, cmd, addr.clone().into()).await?
        }
    }
    if let Some(cfg) = config.authenticator {
        if !cfg.disabled {
            let cmd = StartCommand {
                node_opts,
                create_subcommand: StartSubCommand::Authenticator {
                    addr: cfg.address,
                    enrollers: cfg.enrollers,
                    project: cfg.project,
                },
            };
            println!("starting authenticator service ...");
            start::start_authenticator_service(ctx, cmd, addr.into()).await?
        }
    }

    Ok(())
}
