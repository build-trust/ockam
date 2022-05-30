use clap::Args;

use crate::util::embedded_node;
use ockam::{route, Context, TcpTransport, TCP};
use ockam_api::cloud::Client;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Id of the space.
    id: String,
}

impl ShowCommand {
    pub fn run(command: ShowCommand) {
        embedded_node(show, command);
    }
}

async fn show(mut ctx: Context, cmd: ShowCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let cloud_address = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, cloud_address), "spaces"]; //TODO

    let mut api = Client::new(route, &ctx).await?;
    let res = api.get_space(&cmd.id).await?;
    println!("{res:?}");

    ctx.stop().await?;
    Ok(())
}
