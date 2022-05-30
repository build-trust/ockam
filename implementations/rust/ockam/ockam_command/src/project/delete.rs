use clap::Args;

use crate::util::embedded_node;
use ockam::{route, Context, TcpTransport, TCP};
use ockam_api::cloud::Client;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Id of the space.
    space_id: String,

    /// Id of the project.
    project_id: String,
}

impl DeleteCommand {
    pub fn run(command: DeleteCommand) {
        embedded_node(delete, command);
    }
}

async fn delete(mut ctx: Context, cmd: DeleteCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let cloud_address = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, cloud_address), "projects"]; //TODO

    let mut api = Client::new(route, &ctx).await?;
    let res = api.delete_project(&cmd.space_id, &cmd.project_id).await?;
    println!("{res:?}");

    ctx.stop().await?;
    Ok(())
}
