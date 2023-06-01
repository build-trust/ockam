use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::Context;
use ockam_api::nodes::models::transport::{TransportList, TransportStatus};

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::util::{api, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP listeners
#[derive(Args, Clone, Debug)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.api_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.api_node);
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::list_tcp_listeners()).await?;
    let res = rpc.parse_response::<TransportList>()?;

    list_listeners(&res.list).await?;

    Ok(())
}

pub async fn list_listeners(list: &[TransportStatus]) -> crate::Result<()> {
    let table = list
        .iter()
        .fold(
            vec![],
            |mut acc,
             TransportStatus {
                 tt,
                 tm,
                 socket_addr,
                 processor_address,
                 flow_control_id,
                 ..
             }| {
                let row = vec![
                    tt.cell(),
                    tm.cell(),
                    socket_addr.cell(),
                    processor_address.cell(),
                    flow_control_id.cell(),
                ];
                acc.push(row);
                acc
            },
        )
        .table()
        .title(vec![
            "Type".cell().bold(true),
            "Mode".cell().bold(true),
            "Address bind".cell().bold(true),
            "Worker address".cell().bold(true),
            "Flow Control Id".cell().bold(true),
        ]);

    print_stdout(table)?;

    Ok(())
}
