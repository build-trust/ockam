use crate::util::OckamConfig;
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,

    /// Select a creation variant
    #[clap(subcommand)]
    pub create_subcommand: CreateTypeCommand,

    /// Give this portal endpoint a name.  If none is provided a
    /// random one will be generated.
    #[clap(short, long)]
    pub alias: Option<String>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CreateTypeCommand {
    /// Create a TCP portal inlet
    TcpInlet {
        /// Portal inlet bind address
        bind: String,
    },
    /// Create a TCP portal outlet
    TcpOutlet {
        /// Portal outlet connection address
        address: String,
    },
}

impl CreateCommand {
    pub fn run(cfg: &mut OckamConfig, command: CreateCommand) {
        let _port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        todo!()
    }
}
