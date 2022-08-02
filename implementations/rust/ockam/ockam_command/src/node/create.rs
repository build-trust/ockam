use clap::Args;
use rand::prelude::random;

use std::{
    env::current_exe,
    fs::OpenOptions,
    net::{IpAddr, SocketAddr},
    process::Command,
    str::FromStr,
    time::Duration,
};

use crate::{
    node::echoer::Echoer,
    node::show::query_status,
    node::uppercase::Uppercase,
    util::{connect_to, embedded_node, find_available_port, OckamConfig},
    CommandGlobalOpts,
};
use ockam::{Context, TcpTransport};
use ockam_api::{
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{NodeManager, NODEMANAGER_ADDR},
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    #[clap(default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,

    /// Spawn a node in the foreground.
    #[clap(display_order = 900, long, short)]
    foreground: bool,

    /// Skip creation of default Vault and Identity
    #[clap(display_order = 901, long, short)]
    skip_defaults: bool,

    /// Specify the API address
    #[clap(default_value = "127.0.0.1:0", long, short)]
    api_address: String,

    #[clap(long, hide = true)]
    no_watchdog: bool,
}
impl CreateCommand {
    pub fn run(opts: CommandGlobalOpts, command: CreateCommand) {
        let cfg = &opts.config;
        let address: SocketAddr = if &command.api_address == "127.0.0.1:0" {
            let port = find_available_port().expect("failed to acquire available port");
            SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), port)
        } else {
            command
                .api_address
                .parse()
                .expect("failed to parse api address")
        };

        let command = CreateCommand {
            api_address: address.to_string(),
            ..command
        };

        if command.foreground {
            // HACK: try to get the current node dir.  If it doesn't
            // exist the user PROBABLY started a non-detached node.
            // Thus we need to create the node dir so that subsequent
            // calls to it don't fail
            if cfg.get_node_dir(&command.node_name).is_err() {
                if let Err(e) = cfg.create_node(&command.node_name, address.port(), 0) {
                    eprintln!(
                        "failed to update node configuration for '{}': {:?}",
                        command.node_name, e
                    );
                    std::process::exit(-1);
                }

                // Save the config update
                if let Err(e) = cfg.atomic_update().run() {
                    eprintln!("failed to update configuration: {}", e);
                    std::process::exit(-1);
                }
            }

            embedded_node(setup, (command, cfg.clone()));
        } else {
            // On systems with non-obvious path setups (or during
            // development) re-executing the current binary is a more
            // deterministic way of starting a node.
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

            // FIXME: not really clear why this is causing issues
            if cfg.port_is_used(address.port()) {
                eprintln!("Another node is listening on the provided port!");
                std::process::exit(-1);
            }

            // First we create a new node in the configuration so that
            // we can ask it for the correct log path, as well as
            // making sure the watchdog can do its job later on.
            if let Err(e) = cfg.create_node(&command.node_name, address.port(), 0) {
                eprintln!(
                    "failed to update node configuration for '{}': {:?}",
                    command.node_name, e
                );
                std::process::exit(-1);
            }

            let (mlog, elog) = cfg.log_paths_for_node(&command.node_name).unwrap();

            let main_log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(mlog)
                .expect("failed to open log path");

            let stderr_log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(elog)
                .expect("failed to open stderr log path");

            let verbose = match opts.global_args.verbose {
                // Enable logs at DEBUG level by default
                0 => "-vv".to_string(),
                // Pass the provided verbosity level to the background node
                v => format!("-{}", "v".repeat(v as usize)),
            };

            let mut args = vec![
                verbose,
                "--no-color".to_string(),
                "node".to_string(),
                "create".to_string(),
                "--api-address".to_string(),
                address.to_string(),
                "--foreground".to_string(),
            ];

            if command.skip_defaults {
                args.push("--skip-defaults".to_string());
            }

            args.push(command.node_name.clone());

            let child = Command::new(ockam)
                .args(args)
                .stdout(main_log_file)
                .stderr(stderr_log_file)
                .spawn()
                .expect("could not spawn node");

            // Update the pid in the config (should we remove this?)
            cfg.update_pid(&command.node_name, child.id() as i32)
                .expect("should never panic");

            // Unless this CLI was called from another watchdog we
            // start the watchdog here
            if !command.no_watchdog {}

            // Save the config update
            if let Err(e) = cfg.atomic_update().run() {
                eprintln!("failed to update configuration: {}", e);
            }

            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));

            println!("\nNode Created!");
            // Then query the node manager for the status
            connect_to(address.port(), cfg.clone(), query_status);
        }
    }
}

async fn setup(ctx: Context, (c, cfg): (CreateCommand, OckamConfig)) -> anyhow::Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let bind = c.api_address;
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&c.node_name).unwrap(); // can't fail because we already checked it
    let node_man = NodeManager::create(
        &ctx,
        c.node_name,
        node_dir,
        c.skip_defaults,
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;
    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    ctx.start_worker("uppercase", Uppercase).await?;
    ctx.start_worker("echoer", Echoer).await?;

    Ok(())
}
