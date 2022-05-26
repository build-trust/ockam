use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the node.
    pub node_name: String,
}

impl ShowCommand {
    pub fn run(command: ShowCommand) {
        println!("showing {:?}", command)
    }
}
