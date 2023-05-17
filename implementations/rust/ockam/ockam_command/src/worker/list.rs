use crate::node::get_node_name;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use ockam::{Context, TcpTransport};
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::workers::WorkerList;
use std::fmt::{Display, Formatter};
use std::time::Duration;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List workers on a node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Node at which to lookup workers (required)
    #[arg(value_name = "NODE", long, display_order = 800)]
    at: Option<String>,
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
    let at = get_node_name(&opts.state, cmd.at.clone())?;
    if let Ok(node_state) = opts.state.nodes.get(&at) {
        let tcp = TcpTransport::create(&ctx).await?;
        let mut rpc = RpcBuilder::new(&ctx, &opts, node_state.name())
            .tcp(&tcp)?
            .build();
        if rpc
            .request_with_timeout(api::list_workers(), Duration::from_millis(1000))
            .await
            .is_ok()
        {
            let workers = rpc.parse_response::<WorkerList>()?;
            println!("Node: {}", &node_state.name());
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
