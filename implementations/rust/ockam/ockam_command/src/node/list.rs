use crate::config::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(_cfg: &mut OckamConfig, command: ListCommand) {
        println!("listing {:?}", command)
    }
}
