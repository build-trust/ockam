use clap::Parser;

use ockam::{Context, TcpTransport};

pub(crate) mod enroll;
pub(crate) mod project;
pub(crate) mod space;

#[derive(Clone, Debug, Parser)]
pub struct CloudCommand {
    #[clap(subcommand)]
    pub subcommand: CloudSubCommand,
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Parser)]
pub enum CloudSubCommand {
    /// Enroll identity in ockam.cloud.
    #[clap(display_order = 1000)]
    Enroll(enroll::EnrollCommandArgs),
    /// Space subcommands.
    #[clap(display_order = 1001)]
    Space(space::SpaceCommand),
    /// Project subcommands.
    #[clap(display_order = 1002)]
    Project(project::ProjectCommand),
}

impl CloudCommand {
    pub async fn run(mut ctx: Context, command: CloudCommand) -> anyhow::Result<()> {
        TcpTransport::create(&ctx).await?;
        match command.subcommand {
            CloudSubCommand::Enroll(arg) => enroll::run(arg, &mut ctx).await,
            CloudSubCommand::Space(arg) => space::run(arg, &mut ctx).await,
            CloudSubCommand::Project(arg) => project::run(arg, &mut ctx).await,
        }?;
        ctx.stop().await?;
        Ok(())
    }
}
