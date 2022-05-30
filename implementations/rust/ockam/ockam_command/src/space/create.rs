use clap::Args;

use crate::util::embedded_node;
use ockam::{route, Context, TcpTransport, TCP};
use ockam_api::cloud::{space::CreateSpace, Client};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Name of the space.
    name: String,
}

impl CreateCommand {
    pub fn run(command: CreateCommand) {
        embedded_node(create, command);
    }
}

async fn create(mut ctx: Context, cmd: CreateCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let cloud_address = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, cloud_address), "spaces"]; //TODO

    let mut api = Client::new(route, &ctx).await?;
    let request = CreateSpace::new(cmd.name);
    let res = api.create_space(request).await?;
    println!("{res:?}");

    ctx.stop().await?;
    Ok(())
}
