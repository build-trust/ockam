use clap::Args;

use crate::docs;

const ENV_INFO: &str = include_str!("./static/env_info.txt");

#[derive(Clone, Debug, Args)]
#[command(about = docs::about("Outputs information about environment variables used by the Ockam CLI"))]
pub struct EnvironmentCommand {}

impl EnvironmentCommand {
    pub fn run(self) -> miette::Result<()> {
        println!("{}", ENV_INFO);
        Ok(())
    }

    pub fn name(&self) -> String {
        "show environment variables".to_string()
    }
}
