use crate::node::default_node_name;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts};
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::workers::WorkerList;
use std::fmt::{Display, Formatter};
use std::time::Duration;

const HELP_DETAIL: &str = "";

/// List workers
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct ListCommand {
    /// Node at which to lookup workers (required)
    #[arg(value_name = "NODE", long, default_value_t = default_node_name(), display_order = 800)]
    at: String,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    if let Ok(node_state) = opts.state.nodes.get(&cmd.at) {
        let tcp = TcpTransport::create(&ctx).await?;
        let mut rpc = RpcBuilder::new(&ctx, &opts, &node_state.config.name)
            .tcp(&tcp)?
            .build();
        if rpc
            .request_with_timeout(api::list_workers(), Duration::from_millis(1000))
            .await
            .is_ok()
        {
            let workers = rpc.parse_response::<WorkerList>()?;
            println!("Node: {}", &node_state.config.name);
            print!("{}", WorkerDisplay(workers))
        }
    }
    Ok(())
}

struct WorkerDisplay<'a>(WorkerList<'a>);

impl Display for WorkerDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0.list.is_empty() {
            writeln!(f, "No workers found.")?;
            return Ok(());
        }

        let sorted: Vec<String> = self.0.list.iter().map(|ws| ws.addr.to_string()).collect();

        writeln!(f, "{:2}Workers:", "")?;
        for (_idx, worker) in sorted.iter().enumerate() {
            writeln!(f, "{:4}{}", "", worker)?;
        }
        Ok(())
    }
}
