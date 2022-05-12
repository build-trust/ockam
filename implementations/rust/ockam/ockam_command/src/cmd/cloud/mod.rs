use clap::Parser;

use ockam::Context;

pub(crate) mod enroll;
// pub(crate) mod project;
// pub(crate) mod space;

#[derive(Clone, Debug, Parser)]
pub struct CloudCommand {
    #[clap(subcommand)]
    pub command: CloudSubCommand,
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Parser)]
pub enum CloudSubCommand {
    /// Enroll identity in ockam.cloud.
    #[clap(display_order = 1000)]
    Enroll(enroll::EnrollCommandArgs),
    // /// Space subcommands.
    // #[clap(display_order = 1001)]
    // Space(space::SpaceCommand),
    // /// Project subcommands.
    // #[clap(display_order = 1002)]
    // Project(project::ProjectCommand),
}

pub async fn run(args: CloudCommand, ctx: Context) -> anyhow::Result<()> {
    match args.command {
        CloudSubCommand::Enroll(arg) => enroll::run(arg, ctx).await,
        // CloudSubCommand::Space(arg) => space::run(arg, ctx).await,
        // CloudSubCommand::Project(arg) => project::run(arg, ctx).await,
    }
}
