use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;

use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

use clap::Args;
use colorful::Colorful;
use ockam_api::nodes::models::portal::OutletList;

use ockam_core::api::Request;
use tokio::sync::Mutex;
use tokio::try_join;

const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Outlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = extract_address_value(&node_name)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        rpc.request(Request::get("/node/outlet")).await?;

        *is_finished.lock().await = true;
        rpc.parse_response::<OutletList>()
    };

    let output_messages = vec![format!(
        "Listing TCP Outlets on node {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (outlets, _) = try_join!(send_req, progress_output)?;

    let list = opts.terminal.build_list(
        &outlets.list,
        &format!("Outlets on Node {node_name}"),
        &format!("No TCP Outlets found on node {node_name}."),
    )?;
    opts.terminal.stdout().plain(list).write_line()?;

    Ok(())
}
