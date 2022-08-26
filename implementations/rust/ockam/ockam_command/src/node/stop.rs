use crate::{
    util::{exitcode, startup},
    CommandGlobalOpts,
};
use clap::Args;
use rand::prelude::random;

#[derive(Clone, Debug, Args)]
pub struct StopCommand {
    /// Name of the node.
    #[clap(default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
    /// Whether to use the SIGTERM or SIGKILL signal to stop the node
    #[clap(long)]
    kill: bool,
}

impl StopCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) {
        let cfg = opts.config;
        match cfg.get_node_pid(&command.node_name) {
            Ok(Some(pid)) => {
                if let Err(e) = startup::stop(pid, command.kill) {
                    eprintln!("{e:?}");
                    std::process::exit(exitcode::OSERR);
                }
            }
            Ok(_) => {
                eprintln!("Node {} is not running!", &command.node_name);
                std::process::exit(exitcode::IOERR);
            }
            Err(_) => {
                eprintln!("Node {} does not exist!", &command.node_name);
                std::process::exit(exitcode::IOERR);
            }
        };
    }
}
