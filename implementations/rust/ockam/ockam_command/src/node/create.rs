use clap::Args;
use std::{env::current_exe, fs::OpenOptions, process::Command, time::Duration};

use crate::{
    node::show::query_status,
    util::{connect_to, embedded_node, OckamConfig, DEFAULT_TCP_PORT},
};
use ockam::{Context, TcpTransport};
use ockam_api::{
    nodes::types::{TransportMode, TransportType},
    nodes::{NodeMan, NODEMAN_ADDR},
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    #[clap(default_value_t = String::from("default"))]
    node_name: String,

    /// Spawn a node in the background.
    #[clap(display_order = 900, long, short)]
    foreground: bool,

    /// Specify the API port
    #[clap(default_value_t = DEFAULT_TCP_PORT, long, short)]
    port: u16,

    #[clap(long, hide = true)]
    no_watchdog: bool,
}
impl CreateCommand {
    pub fn run(cfg: &OckamConfig, command: CreateCommand) {
        if command.foreground {
            // HACK: try to get the current node dir.  If it doesn't
            // exist the user PROBABLY started a non-detached node.
            // Thus we need to create the node dir so that subsequent
            // calls to it don't fail
            if cfg.get_node_dir(&command.node_name).is_err() {
                if let Err(e) = cfg.create_node(&command.node_name, command.port, 0) {
                    eprintln!(
                        "failed to update node configuration for '{}': {:?}",
                        command.node_name, e
                    );
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
            if cfg.port_is_used(command.port) {
                eprintln!("Another node is listening on the provided port!");
                std::process::exit(-1);
            }

            // First we create a new node in the configuration so that
            // we can ask it for the correct log path, as well as
            // making sure the watchdog can do its job later on.
            if let Err(e) = cfg.create_node(&command.node_name, command.port, 0) {
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

            let child = Command::new(ockam)
                .args([
                    "-vv", // Enable logs at DEBUG level
                    "node",
                    "create",
                    "--port",
                    &command.port.to_string(),
                    "--foreground",
                    &command.node_name,
                ])
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

            // Then query the node manager for the status
            connect_to(command.port, (), query_status);
        }
    }
}

async fn setup(ctx: Context, (c, cfg): (CreateCommand, OckamConfig)) -> anyhow::Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let bind = format!("0.0.0.0:{}", c.port);
    tcp.listen(&bind).await?;

    let node_dir = cfg.get_node_dir(&c.node_name).unwrap(); // can't fail because we already checked it
    let node_man = NodeMan::create(
        &ctx,
        c.node_name,
        node_dir,
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;
    ctx.start_worker(NODEMAN_ADDR, node_man).await?;

    Ok(())
}
