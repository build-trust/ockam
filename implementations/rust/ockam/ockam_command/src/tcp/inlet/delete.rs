use crate::node::{get_node_name, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_core::api::{Request, RequestBuilder};

/// Delete a TCP Inlet
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name assigned to inlet that will be deleted
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp inlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, DeleteCommand),
) -> crate::Result<()> {
    let alias = cmd.alias.clone();
    let node_name = get_node_name(&options.state, cmd.node_opts.api_node.clone())?;
    let node = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;
    rpc.request(make_api_request(cmd)?).await?;

    rpc.is_ok()?;

    options
        .terminal
        .stdout()
        .plain(format!(
            "{} TCP Inlet with alias {alias} on Node {node} has been deleted.",
            "✔︎".light_green(),
        ))
        .machine(&alias)
        .json(serde_json::json!({ "tcp-inlet": { "alias": alias, "node": node } }))
        .write_line()?;
    Ok(())
}

/// Construct a request to delete a tcp inlet
fn make_api_request<'a>(cmd: DeleteCommand) -> Result<RequestBuilder<'a>> {
    let alias = cmd.alias;
    let request = Request::delete(format!("/node/inlet/{alias}"));
    Ok(request)
}
