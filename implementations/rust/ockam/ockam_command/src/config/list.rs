use crate::{CommandGlobalOpts, OckamConfig};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(_: CommandGlobalOpts, _: ListCommand) {
        OckamConfig::values()
            .iter()
            .for_each(|val| println!("{}", val));
    }
}
