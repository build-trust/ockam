use clap::Args;
use std::{env::current_exe, fs::OpenOptions, process::Command, time::Duration};

use crate::{
    node::show::query_status,
    util::{connect_to, embedded_node, OckamConfig, DEFAULT_TCP_PORT},
};
use ockam::authenticated_storage::InMemoryStorage;
use ockam::{vault::Vault, AsyncTryClone, Context, TcpTransport};
use ockam_api::{
    auth,
    identity::IdentityService,
    nodes::types::{TransportMode, TransportType},
    nodes::NodeMan,
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the node.
    #[clap(default_value_t = String::from("default"))]
    node_name: String,

    /// Spawn a node in the background.
    #[clap(display_order = 900, long, short)]
    spawn: bool,

    #[clap(default_value_t = DEFAULT_TCP_PORT, long, short)]
    port: u16,
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) {
        if command.spawn {
            // On systems with non-obvious path setups (or during
            // development) re-executing the current binary is a more
            // deterministic way of starting a node.
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                // FIXME: slugify the node name
                .open(&cfg.log_path.join(format!("{}.log", command.node_name)))
                .expect("failed to open log path");

            let child = Command::new(ockam)
                .args([
                    "-vv", // Enable logs at DEBUG level
                    "node",
                    "create",
                    "--port",
                    &command.port.to_string(),
                    &command.node_name,
                ])
                .stdout(log_file)
                .spawn()
                .expect("could not spawn node");

            if let Err(e) = cfg.create_node(&command.node_name, command.port, child.id() as i32) {
                eprintln!(
                    "failed to update node configuration for '{}': {:?}",
                    command.node_name, e
                );
                std::process::exit(-1);
            }
            cfg.save();

            // Wait a bit
            std::thread::sleep(Duration::from_millis(500));

            // Then query the node manager for the status
            connect_to(command.port, (), query_status);
        } else {
            // FIXME: not really clear why this is causing issues
            // if cfg.port_is_used(command.port) {
            //     eprintln!("Another node is listening on the provided port!");
            //     std::process::exit(-1);
            // }

            embedded_node(setup, command);
        }
    }
}

async fn setup(ctx: Context, c: CreateCommand) -> anyhow::Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    let bind = format!("0.0.0.0:{}", c.port);
    tcp.listen(&bind).await?;

    let s = InMemoryStorage::new();
    ctx.start_worker("authenticated", auth::Server::new(s))
        .await?;

    // TODO: put that behind some flag or configuration
    let vault = Vault::create();
    IdentityService::create(&ctx, "identity_service", vault.async_try_clone().await?).await?;

    ctx.start_worker(
        "_internal.nodeman",
        NodeMan::new(
            c.node_name,
            (TransportType::Tcp, TransportMode::Listen, bind),
            tcp,
        ),
    )
    .await?;

    Ok(())
}
