use clap::Args;

use crate::util::embedded_node;
use ockam::{route, Context, TcpTransport, TCP};
use ockam_api::cloud::{project::CreateProject, Client};

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Id of the space the project belongs to.
    space_id: String,

    /// Name of the project.
    project_name: String,

    /// Services enabled for this project.
    services: Vec<String>,
}

impl CreateCommand {
    pub fn run(command: CreateCommand) {
        embedded_node(create, command);
    }
}

async fn create(mut ctx: Context, cmd: CreateCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let cloud_address = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, cloud_address), "projects"]; //TODO

    let mut api = Client::new(route, &ctx).await?;
    let request = CreateProject::new(cmd.project_name, &cmd.services);
    let res = api.create_project(&cmd.space_id, request).await?;
    println!("{res:?}");

    ctx.stop().await?;
    Ok(())
}
