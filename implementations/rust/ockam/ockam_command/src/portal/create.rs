use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::Args;
use ockam::{Context, Route, TCP};
use ockam_api::{nodes::types::TransportStatus, Status};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Create a listening transport
    #[clap(short, long, conflicts_with("outlet"))]
    pub inlet: bool,

    /// Create a connection transport
    #[clap(short, long, conflicts_with("inlet"))]
    pub outlet: bool,

    /// Transport connection or bind address
    pub address: String,

    /// Give this portal endpoint a name.  If none is provided a
    /// random one will be generated.
    pub alias: Option<String>,
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

        todo!()
    }
}
