use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::Args;
use ockam::Context;
use ockam_api::Status;
use ockam_core::Route;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Override the default API node
    #[clap(short, long)]
    pub api_node: Option<String>,
}

impl CreateCommand {
    pub fn run(cfg: &OckamConfig, command: CreateCommand) -> anyhow::Result<()> {
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };

        connect_to(port, command, create_identity);

        Ok(())
    }
}

pub async fn create_identity(
    ctx: Context,
    _cmd: CreateCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append("_internal.nodeman"),
            api::create_identity()?,
        )
        .await?;

    let response = api::parse_create_identity_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            eprintln!("Identity created!",)
        }
        _ => {
            eprintln!("An error occurred while creating Identity",)
        }
    }

    stop_node(ctx).await
}
