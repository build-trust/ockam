use crate::util::{api, connect_to, stop_node, OckamConfig, AddonCommand};
use clap::Args;
use ockam::{Context, Route, TCP};
use ockam_api::{nodes::types::TransportStatus, Status};

// Creating transports has two sub-commands
//
// tcp-listener
// tcp-connection
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Specify the type of transport to create
    pub addon_command: AddonCommand,
    // /// Create a listening transport
    // #[clap(short, long, conflicts_with("connect"))]
    // pub listen: bool,

    // /// Create a connection transport
    // #[clap(short, long, conflicts_with("listen"))]
    // pub connect: bool,

    // /// Create a TCP transport
    // #[clap(long)]
    // pub tcp: bool,

    // /// Transport connection or bind address
    // pub address: String,
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

        // if !command.connect && !command.listen {
        //     eprintln!("Either --connect or --listen must be provided!");
        //     std::process::exit(-1);
        // }

        println!("Self: {:?}", command);

        // connect_to(port, command, create_transport);
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
