use crate::help;
use crate::node::NodeOpts;
use crate::util::{api, connect_to, exitcode};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::Status;
use ockam_core::Route;

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) -> anyhow::Result<()> {
        let cfg = options.config;
        let port = cfg.get_node_port(&self.node_opts.api_node).unwrap();

        connect_to(port, self, create_identity);

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
            base_route.modify().append(NODEMANAGER_ADDR),
            api::create_identity()?,
        )
        .await?;

    let (response, result) = api::parse_create_identity_response(&resp)?;

    match response.status() {
        Some(Status::Ok) => {
            println!("Identity {} created!", result.identity_id)
        }
        _ => {
            eprintln!("An error occurred while creating Identity",);
            std::process::exit(exitcode::CANTCREAT);
        }
    }

    Ok(())
}
