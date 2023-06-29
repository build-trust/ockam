use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_core::api::{Request, RequestBuilder};

use crate::{CommandGlobalOpts, docs, fmt_ok};
use crate::node::get_node_name;
use crate::Result;
use crate::util::{extract_address_value, node_rpc, Rpc};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DeleteCommand {
    /// Name assigned to Relay that will be deleted
    #[arg(display_order = 900, required = true)]
    relay_name: String,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let relay_name = cmd.relay_name.clone();
    let at = get_node_name(&options.state, &cmd.at);
    let node = extract_address_value(&at)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;
    rpc.request(make_api_request(cmd)?).await?;

    rpc.is_ok()?;

    options
        .terminal
        .stdout()
        .plain(fmt_ok!(
            "Relay with name {} on Node {} has been deleted.",
            relay_name,
            node
        ))
        .machine(&relay_name)
        .json(serde_json::json!({ "forwarder": { "name": relay_name, "node": node } }))
        .write_line()?;
    Ok(())
}

/// Construct a request to delete a relay
fn make_api_request<'a>(cmd: DeleteCommand) -> Result<RequestBuilder<'a>> {
    let request = Request::delete(format!("/node/forwarder/{}", cmd.relay_name));
    Ok(request)
}
