use crate::util::{api, connect_to, stop_node};
use crate::CommandGlobalOpts;
use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::{Context, Route};
use ockam_api::nodes::{
    types::{TransportList, TransportStatus},
    NODEMAN_ADDR,
};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Name of the node.
    #[clap(short, long)]
    pub api_node: Option<String>,
}

impl ListCommand {
    pub fn run(opts: CommandGlobalOpts, command: ListCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&command.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(-1);
            }
        };
        connect_to(port, (), query_transports);
    }
}

pub async fn query_transports(ctx: Context, _: (), mut base_route: Route) -> anyhow::Result<()> {
    let resp: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMAN_ADDR),
            api::query_transports()?,
        )
        .await
        .unwrap();

    let TransportList { list, .. } = api::parse_transport_list(&resp)?;

    let table = list
        .iter()
        .fold(
            vec![],
            |mut acc,
             TransportStatus {
                 tt,
                 tm,
                 payload,
                 tid,
                 ..
             }| {
                let row = vec![tid.cell(), tt.cell(), tm.cell(), payload.cell()];
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
        ]);

    if let Err(e) = print_stdout(table) {
        eprintln!("failed to print node status: {}", e);
    }

    stop_node(ctx).await
}
