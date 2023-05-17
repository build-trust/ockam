use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::Context;
use ockam_api::nodes::models::transport::{TransportList, TransportStatus};

use crate::node::{get_node_name, NodeOpts};
use crate::util::{api, node_rpc, Rpc};
use crate::CommandGlobalOpts;

#[derive(Args, Clone, Debug)]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
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
    let node_name = get_node_name(&opts.state, cmd.node_opts.api_node.clone())?;
    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    rpc.request(api::list_tcp_listeners()).await?;
    let res = rpc.parse_response::<TransportList>()?;

    list_listeners(&res.list).await?;

    Ok(())
}

pub async fn list_listeners<'a>(list: &[TransportStatus<'a>]) -> crate::Result<()> {
    let table = list
        .iter()
        .fold(
            vec![],
            |mut acc,
             TransportStatus {
                 tt,
                 tm,
                 socket_addr,
                 worker_addr,
                 tid,
                 ..
             }| {
                let row = vec![
                    tid.cell(),
                    tt.cell(),
                    tm.cell(),
                    socket_addr.cell(),
                    worker_addr.cell(),
                ];
                acc.push(row);
                acc
            },
        )
        .table()
        .title(vec![
            "Transport ID".cell().bold(true),
            "Transport Type".cell().bold(true),
            "Mode".cell().bold(true),
            "Address bind".cell().bold(true),
            "Worker address".cell().bold(true),
        ]);

    print_stdout(table)?;

    Ok(())
}
