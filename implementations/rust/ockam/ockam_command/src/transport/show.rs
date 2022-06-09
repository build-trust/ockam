use crate::util::{api, connect_to, stop_node, OckamConfig};
use clap::Args;
use cli_table::{print_stdout, Cell, Style, Table};
use ockam::{Context, Route};
use ockam_api::nodes::types::{TransportList, TransportStatus};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Name of the node.
    pub node_name: Option<String>,
}

impl ShowCommand {
    pub fn run(cfg: &mut OckamConfig, command: ShowCommand) {
        let port = match cfg.select_node(&command.node_name) {
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
            base_route.modify().append("_internal.nodeman"),
            api::query_transports()?,
        )
        .await
        .unwrap();

    let TransportList { list, .. } = api::parse_transports(&resp)?;

    let table = list
        .iter()
        .fold(vec![], |mut acc, TransportStatus { tt, tm, addr, .. }| {
            let row = vec![tt.cell(), tm.cell(), addr.cell()];
            acc.push(row);
            acc
        })
        .table()
        .title(vec![
            "Transport Type".cell().bold(true),
            "Mode".cell().bold(true),
            "Address bind".cell().bold(true),
        ]);

    if let Err(e) = print_stdout(table) {
        eprintln!("failed to print node status: {}", e);
    }

    stop_node(ctx).await
}
