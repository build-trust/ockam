use clap::Args;

const ENV_INFO: &str = include_str!("./static/env_info.txt");

/// Outputs information about environment variables used by the Ockam CLI
#[derive(Clone, Debug, Args)]
pub struct EnvironmentCommand {}

impl EnvironmentCommand {
    pub fn run(self) {
        println!("{}", ENV_INFO);
    }
}
