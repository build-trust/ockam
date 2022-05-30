mod enroll;
mod project;
mod space;

use enroll::EnrollCommand;
use project::ProjectCommand;
use space::SpaceCommand;

use clap::{Args, Subcommand};

#[derive(Clone, Debug, Args)]
pub struct CloudCommand {
    #[clap(subcommand)]
    pub subcommand: CloudSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CloudSubcommand {
    /// Enroll with Ockam Cloud
    #[clap(display_order = 900)]
    Enroll(EnrollCommand),

    /// Create, update and delete projects in Ockam Cloud
    #[clap(display_order = 900)]
    Project(ProjectCommand),

    /// Create, update and delete spaces in Ockam Cloud
    #[clap(display_order = 900)]
    Space(SpaceCommand),
}

impl CloudCommand {
    pub fn run(command: CloudCommand) {
        match command.subcommand {
            CloudSubcommand::Enroll(command) => EnrollCommand::run(command),
            CloudSubcommand::Project(command) => ProjectCommand::run(command),
            CloudSubcommand::Space(command) => SpaceCommand::run(command),
        }
    }
}
