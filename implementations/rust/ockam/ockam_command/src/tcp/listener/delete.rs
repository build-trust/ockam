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

    /// Tcp Listener ID
    pub id: String,

    /// Force this operation: delete the API transport if requested
    #[clap(long)]
    pub force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let node = get_final_element(&self.node_opts.api_node);
        let port = cfg.get_node_port(node);
        connect_to(port, self, delete_listener);
    }
}

pub async fn delete_listener(
    ctx: Context,
    cmd: DeleteCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = match ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::delete_tcp_listener(&cmd)?,
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
        Some(Status::Ok) => println!("Tcp listener `{}` successfully delete", cmd.id),
        _ => {
            eprintln!("Failed to delete tcp listener");
            if !cmd.force {
                eprintln!("You may have to provide --force to delete the API transport");
                std::process::exit(exitcode::UNAVAILABLE);
            }
        }
    }
    Ok(())
}
