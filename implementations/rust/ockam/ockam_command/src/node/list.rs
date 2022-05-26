use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(command: ListCommand) {
        println!("listing {:?}", command)
    }
}
