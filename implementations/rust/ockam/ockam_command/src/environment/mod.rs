use clap::Args;

const ENVTEXT: &str = include_str!("./static/envtext.txt");

// Outputs information about environment variables used by Ockam
#[derive(Clone, Debug, Args)]
pub struct EnvironmentCommand {}

impl EnvironmentCommand {
    pub fn run(self) {
        println!("{}", ENVTEXT);
    }
}

