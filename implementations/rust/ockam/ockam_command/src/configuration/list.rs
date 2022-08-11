use crate::CommandGlobalOpts;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, _: ListCommand) {
        let lookup = opts.config.get_node_lookup();

        for (alias, addr) in lookup {
            println!("Node:    {}\nAddress: {}\n", alias, addr);
        }
    }
}
