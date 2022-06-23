use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::{Args, Subcommand};
use ockam::{Context, Route, TCP};
use ockam_api::{
    nodes::{types::TransportStatus, NODEMAN_ADDR},
    Status,
};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Select a creation variant
    #[clap(subcommand)]
    pub create_subcommand: CreateTypeCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateTypeCommand {
    /// Create a TCP transport listener
    ///
    /// Listens for incoming TCP connections on the given bind address
    /// and port
    TcpListener {
        /// Transport connection or bind address
        bind: String,
    },
    /// Create a TCP transport connector
    ///
    /// Attempts to connect to an existing TCP transport listener on
    /// the given peer address and port
    TcpConnector {
        /// Transport connection or bind address
        address: String,
    },
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) {
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command.clone(), create_transport);

        // Update the config.  We can probably assume that everything
        // went OK if we reach this point because embedded_node
        // crashes the process if something went wrong?  But idk,
        // still bad and should be fixed
        let node = command.api_node.unwrap_or_else(|| cfg.api_node.clone());
        match command.create_subcommand {
            CreateTypeCommand::TcpConnector { address } => {
                cfg.add_transport(&node, false, true, address)
            }
            CreateTypeCommand::TcpListener { bind } => cfg.add_transport(&node, true, true, bind),
        };

        if let Err(e) = cfg.atomic_update().run() {
            eprintln!("failed to update configuration: {}", e);
        }
    }
}

pub async fn create_transport(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::create_transport(&cmd)?,
        )
        .await
        .unwrap();

    let (response, TransportStatus { payload, .. }) = api::parse_transport_status(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            let r: Route = base_route
                .modify()
                .pop_back()
                .append_t(TCP, payload.to_string())
                .into();

            eprintln!(
                "Transport created! You can send messages to it via this route:\n{}`",
                r
            )
        }
        _ => {
            eprintln!(
                "An error occurred while creating the transport: {}",
                payload
            )
        }
    }

    stop_node(ctx).await
}
