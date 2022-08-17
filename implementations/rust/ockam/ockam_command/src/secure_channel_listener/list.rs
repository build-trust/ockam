use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::{Context, Route};
use ockam_api::nodes::NODEMANAGER_ADDR;

use crate::{
    node::NodeOpts,
    util::{
        api::{self, parse_list_secure_channel_listener_response},
        connect_to, exitcode, stop_node,
    },
    CommandGlobalOpts,
};

#[derive(Args, Clone, Debug)]
pub struct ListCommand {
    /// Node of which secure listeners shall be listed
    #[clap(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, command: Self) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, command, list_secure_channel_listeners);
        Ok(())
    }
}

pub async fn list_secure_channel_listeners(
    ctx: Context,
    cmd: ListCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::list_secure_channel_listener()?,
        )
        .await?;

    let secure_channel_address_listener_list = parse_list_secure_channel_listener_response(&resp)?;

    let title_name = "Secure Channel Listener Address for ".to_owned() + &cmd.node_opts.api_node;

    let table = secure_channel_address_listener_list
        .list
        .iter()
        .fold(vec![], |mut acc, x| {
            let row = vec![x];
            acc.push(row);
            acc
        })
        .table()
        .title(vec![title_name.as_str().cell().bold(true)]);

    if let Err(e) = print_stdout(table) {
        eprintln!("failed to print secure channel listeners: {}", e);
        std::process::exit(exitcode::IOERR);
    }

    stop_node(ctx).await
}
