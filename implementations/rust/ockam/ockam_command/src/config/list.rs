use crate::util::OckamConfig;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(_: &mut OckamConfig, _: ListCommand) {
        OckamConfig::values()
            .iter()
            .for_each(|val| println!("{}", val));
    }
}
