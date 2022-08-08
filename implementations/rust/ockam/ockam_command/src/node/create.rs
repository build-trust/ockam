use clap::Args;
use rand::prelude::random;

use std::sync::Arc;
use std::{
    env::current_exe,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use crate::util::exitcode;
use crate::{
    node::show::query_status,
    util::{
        connect_to, embedded_node, find_available_port, startup, ComposableSnippet, OckamConfig,
        Operation,
    },
    CommandGlobalOpts, HELP_TEMPLATE,
};
use ockam::identity::Identity;
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::nodes::IdentityOverride;
use ockam_api::{
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{NodeManager, NODEMANAGER_ADDR},
};
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
    node_name: String,

    /// Run the node in foreground.
    #[clap(display_order = 900, long, short)]
    foreground: bool,

    /// TCP listener address
    #[clap(
        display_order = 900,
        long,
        short,
        name = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    tcp_listener_address: String,

    /// Skip creation of default Vault and Identity
    #[clap(long, short, hide = true)]
    skip_defaults: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[clap(long, hide = true)]
    launch_config: Option<PathBuf>,

    #[clap(long, hide = true)]
    no_watchdog: bool,
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
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let address: SocketAddr = if &command.tcp_listener_address == "127.0.0.1:0" {
            let port = find_available_port().expect("failed to acquire available port");
            SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), port)
        } else {
            command
                .tcp_listener_address
                .parse()
                .expect("failed to parse tcp listener address")
        };

        let verbose = opts.global_args.verbose;
        let command = CreateCommand {
            tcp_listener_address: address.to_string(),
            ..command
        };

        if command.foreground {
            // HACK: try to get the current node dir.  If it doesn't
            // exist the user PROBABLY started a non-detached node.
            // Thus we need to create the node dir so that subsequent
            // calls to it don't fail
            if cfg.get_node_dir(&command.node_name).is_err() {
                println!("Creating node directory...");
                if let Err(e) = cfg.create_node(&command.node_name, address, verbose) {
                    eprintln!(
                        "failed to update node configuration for '{}': {}",
                        command.node_name, e
                    );
                    std::process::exit(exitcode::CANTCREAT);
                }

                // Save the config update
                if let Err(e) = cfg.atomic_update().run() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(exitcode::IOERR);
                }
            }

            if let Err(e) = embedded_node(setup, (command, cfg.clone())) {
                eprintln!("Ockam node failed: {:?}", e,);
            }
        } else {
            // On systems with non-obvious path setups (or during
            // development) re-executing the current binary is a more
            // deterministic way of starting a node.
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

            // FIXME: not really clear why this is causing issues
            if cfg.port_is_used(address.port()) {
                eprintln!("Another node is listening on the provided port!");
                std::process::exit(exitcode::IOERR);
            }

            // First we create a new node in the configuration so that
            // we can ask it for the correct log path, as well as
            // making sure the watchdog can do its job later on.
            if let Err(e) = cfg.create_node(&command.node_name, address, verbose) {
                eprintln!(
                    "failed to update node configuration for '{}': {}",
                    command.node_name, e
                );
                std::process::exit(exitcode::CANTCREAT);
            }

            // Construct the arguments list and re-execute the ockam
            // CLI in foreground mode to start the newly created node
            startup::spawn_node(
                &ockam,
                &opts.config,
                verbose,
                command.skip_defaults,
                &command.node_name,
                &command.tcp_listener_address,
            );

            let composite = (&command).into();
            let startup_cfg = cfg.get_startup_cfg(&command.node_name).unwrap();
            startup_cfg.writelock_inner().commands = vec![composite].into();

            // Save the config update
            if let Err(e) = startup_cfg.atomic_update().run() {
                eprintln!("failed to update configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }

            // Unless this CLI was called from another watchdog we
            // start the watchdog here
            if !command.no_watchdog {}

            // Save the config update
            if let Err(e) = cfg.atomic_update().run() {
                eprintln!("failed to update configuration: {}", e);
                std::process::exit(exitcode::IOERR);
            }

            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));

            println!("\nNode Created!");
            // Then query the node manager for the status
            connect_to(
                address.port(),
                (cfg.clone(), command.node_name),
                query_status,
            );
        }
    }
}

async fn create_identity_override(
    ctx: &Context,
    cfg: &OckamConfig,
) -> anyhow::Result<IdentityOverride> {
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

async fn setup(ctx: Context, (c, cfg): (CreateCommand, OckamConfig)) -> anyhow::Result<()> {
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
        c.node_name,
        node_dir,
        identity_override,
        c.skip_defaults,
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;

    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    Ok(())
}
