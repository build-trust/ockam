use clap::Args;
use rand::prelude::random;

use anyhow::{anyhow, Context as _, Result};
use std::sync::Arc;
use std::{
    env::current_exe,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::project::ProjectInfo;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::service::start::{self, StartCommand, StartSubCommand};
use crate::util::{bind_to_port_check, exitcode};
use crate::{
    help,
    node::show::query_status,
    node::HELP_DETAIL,
    util::{
        connect_to, embedded_node, find_available_port, startup, ComposableSnippet, OckamConfig,
        Operation,
    },
    CommandGlobalOpts,
};
use ockam::identity::{Identity, PublicIdentity};
use ockam::{Address, AsyncTryClone, NodeBuilder, TCP};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::error::ApiError;
use ockam_api::nodes::IdentityOverride;
use ockam_api::{
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{NodeManager, NODEMANAGER_ADDR},
};
use ockam_core::LOCAL;
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use tokio::fs;

/// Create Nodes
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
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

    /// ockam_command started a child process to run this node in foreground.
    #[clap(display_order = 900, long, hide = true)]
    pub child_process: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[clap(long, hide = true)]
    pub launch_config: Option<PathBuf>,

    #[clap(long, hide = true)]
    pub no_watchdog: bool,

    #[clap(long, hide = true)]
    pub project: Option<PathBuf>,
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
                if let Err(e) = cfg.atomic_update().run() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }

            if let Err(e) = run_background_node(cmd, addr, cfg.clone()) {
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
            connect_to(addr.port(), (cfg.clone(), cmd.node_name), query_status);
        }
    }

    pub async fn create_background_node(
        ctx: Context,
        (opts, cmd, addr): (CommandGlobalOpts, CreateCommand, SocketAddr),
    ) -> crate::Result<()> {
        let verbose = opts.global_args.verbose;
        let cfg = &opts.config;

        // On systems with non-obvious path setups (or during
        // development) re-executing the current binary is a more
        // deterministic way of starting a node.
        let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

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
        if let Err(e) = cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
            std::process::exit(exitcode::IOERR);
        }

        create_default_identity_if_needed(&ctx, cfg.clone()).await?;

        // Construct the arguments list and re-execute the ockam
        // CLI in foreground mode to start the newly created node
        startup::spawn_node(
            &ockam,
            &opts.config,
            verbose,
            cmd.skip_defaults,
            &cmd.node_name,
            &cmd.tcp_listener_address,
            cmd.project.as_deref(),
        );

        let composite = (&cmd).into();
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

async fn create_default_identity_if_needed(ctx: &Context, cfg: OckamConfig) -> Result<()> {
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

    cfg.atomic_update().run()?;

    Ok(())
}

async fn get_identity_override(ctx: &Context, cfg: &OckamConfig) -> Result<IdentityOverride> {
    // Get default root vault
    let default_vault_path = cfg
        .get_default_vault_path()
        .ok_or_else(|| ApiError::generic("Default vault was not found"))?;

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let vault = Vault::new(Some(Arc::new(storage)));

    // Get default root identity
    let default_identity = cfg
        .get_default_identity()
        .ok_or_else(|| ApiError::generic("Default identity was not found"))?;
    // Just to check validity
    Identity::import(ctx, &default_identity, &vault).await?;

    Ok(IdentityOverride {
        identity: default_identity,
        vault_path: default_vault_path,
    })
}

fn run_background_node(c: CreateCommand, addr: SocketAddr, cfg: OckamConfig) -> Result<()> {
    let (mut ctx, mut executor) = NodeBuilder::without_access_control().no_logging().build();

    executor
        .execute(async move {
            let v = run_background_node_impl(&mut ctx, c, addr, cfg).await;

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
) -> Result<()> {
    // This node was initially created as a foreground node
    if !c.child_process {
        create_default_identity_if_needed(ctx, cfg.clone()).await?;
    }

    let identity_override = if c.skip_defaults {
        None
    } else {
        Some(get_identity_override(ctx, &cfg).await?)
    };

    if let Some(path) = &c.project {
        add_project_authority(path, &c.node_name, &cfg).await?
    }

    let tcp = TcpTransport::create(ctx).await?;
    let bind = c.tcp_listener_address;
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&c.node_name)?;
    let mut node_man = NodeManager::create(
        ctx,
        c.node_name.clone(),
        node_dir,
        identity_override,
        c.skip_defaults || c.launch_config.is_some(),
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp.async_try_clone().await?,
    )
    .await?;

    node_man
        .configure_authorities(&cfg.authorities(&c.node_name)?.snapshot())
        .await?;

    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    if let Some(path) = c.launch_config {
        let node_opts = super::NodeOpts {
            api_node: c.node_name,
        };
        start_services(ctx, &tcp, &path, addr, node_opts).await?
    }

    Ok(())
}

async fn add_project_authority<P>(path: P, node: &str, cfg: &OckamConfig) -> Result<()>
where
    P: AsRef<Path>,
{
    let s = fs::read_to_string(path.as_ref()).await?;
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
            let ids = cfg.authorized_identifiers;
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
