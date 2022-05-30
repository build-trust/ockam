use clap::Args;

use crate::util::embedded_node;
use ockam::{route, Context, TcpTransport, TCP};
use ockam_api::cloud::Client;

#[derive(Clone, Debug, Args)]
pub struct ListCommand;

impl ListCommand {
    pub fn run(command: ListCommand) {
        embedded_node(list, command);
    }
}

async fn list(mut ctx: Context, _cmd: ListCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let cloud_address = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, cloud_address), "spaces"]; //TODO

    let mut api = Client::new(route, &ctx).await?;
    let res = api.list_spaces().await?;
    println!("{res:?}");

    ctx.stop().await?;
    Ok(())
}
