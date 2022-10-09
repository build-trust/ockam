use clap::Args;
use ockam::{Context, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Response, Status};

use crate::util::extract_address_value;
use crate::{
    node::NodeOpts,
    util::{api, connect_to, exitcode},
    CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Connection ID
    pub id: String,

    /// Force this operation: delete the API transport if requested
    #[arg(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let node =
            extract_address_value(&self.node_opts.api_node).unwrap_or_else(|_| "".to_string());
        let port = cfg.get_node_port(&node).unwrap();
        connect_to(port, self, delete_connection);
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
