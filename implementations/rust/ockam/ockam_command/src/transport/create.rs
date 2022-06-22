use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::{Args, Subcommand};
use ockam::{Context, Route, TCP};
use ockam_api::{nodes::types::TransportStatus, Status};

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
    /// Create a TCP listener transport
    TcpListener {
        /// Transport connection or bind address
        bind: String,
    },
    /// Create a TCP connector transport
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

        connect_to(port, command, create_transport);
    }
}

pub async fn create_transport(
    ctx: Context,
    cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
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
