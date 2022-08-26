use clap::Args;
use ockam::{Context, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Response, Status};

use crate::util::get_final_element;
use crate::{
    node::NodeOpts,
    util::{api, connect_to, exitcode},
    CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    /// Tcp Connection ID
    pub id: String,

    /// Force this operation: delete the API transport if requested
    #[clap(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(opts: CommandGlobalOpts, command: DeleteCommand) {
        let cfg = &opts.config;
        let node = get_final_element(&command.node_opts.api_node);
        let port = match cfg.select_node(node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available. Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, command, delete_connection);
    }
}

pub async fn delete_connection(
    ctx: Context,
    cmd: DeleteCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = match ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::delete_tcp_connection(&cmd)?,
        )
        .await
    {
        Ok(sr_msg) => sr_msg,
        Err(e) => {
            eprintln!("Wasn't able to send or receive `Message`: {}", e);
            std::process::exit(exitcode::IOERR);
        }
    };
    let r: Response = api::parse_response(&resp)?;

    match r.status() {
        Some(Status::Ok) => println!("Tcp connection `{}` successfully delete", cmd.id),
        _ => {
            eprintln!("Failed to delete tcp connection");
            if !cmd.force {
                eprintln!("You may have to provide --force to delete the API transport");
                std::process::exit(exitcode::UNAVAILABLE);
            }
        }
    }
    Ok(())
}
