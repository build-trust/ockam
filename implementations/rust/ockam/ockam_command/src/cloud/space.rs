use clap::Parser;

use ockam::TCP;
use ockam::{Context, TcpTransport};
use ockam_api::cloud::space::CreateSpace;
use ockam_core::route;

use crate::old::storage;

pub async fn run(args: SpaceCommand, ctx: &mut Context) -> anyhow::Result<()> {
    storage::ensure_identity_exists(true)?;
    let _ockam_dir = storage::get_ockam_dir()?;

    TcpTransport::create(ctx).await?;

    let addr = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, addr), "spaces"]; //TODO
    let mut api_client = ockam_api::cloud::Client::new(route, ctx).await?;

    match args.command {
        SpacesSubCommand::Create { name: space_name } => {
            let res = api_client
                .create_space(CreateSpace::new(space_name))
                .await?;
            println!("{res:?}")
        }
        SpacesSubCommand::List => {
            let res = api_client.list_spaces().await?;
            println!("{res:?}")
        }
        SpacesSubCommand::Show { id: space_id } => {
            let res = api_client.get_space(&space_id).await?;
            println!("{res:?}")
        }
        SpacesSubCommand::Delete { id: space_id } => {
            let res = api_client.delete_space(&space_id).await?;
            println!("{res:?}")
        }
    };
    ctx.stop().await?;
    Ok(())
}

#[derive(Clone, Debug, Parser)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    pub command: SpacesSubCommand,
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Parser)]
pub enum SpacesSubCommand {
    /// Creates a new space.
    #[clap(display_order = 1000)]
    Create {
        /// Name of the space.
        name: String,
    },
    /// List all spaces.
    #[clap(display_order = 1001)]
    List,
    /// Shows a single space.
    #[clap(display_order = 1002)]
    Show {
        /// Id of the space.
        id: String,
    },
    /// Delete a space.
    #[clap(display_order = 1003)]
    Delete {
        /// Id of the space.
        id: String,
    },
}
