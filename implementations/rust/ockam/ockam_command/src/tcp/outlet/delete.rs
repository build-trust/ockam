use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_core::api::{Request, RequestBuilder};

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    /// Name assigned to outlet that will be deleted
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp outlet. If none are provided, the default node will be used
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

    let node = extract_address_value(&cmd.node_opts.api_node)?;
    let mut rpc = Rpc::background(&ctx, &options, &node)?;
    rpc.request(make_api_request(cmd)?).await?;

    rpc.is_ok()?;

    options
        .terminal
        .stdout()
        .plain(format!(
            "{} TCP Outlet with alias {alias} on Node {node} has been deleted.",
            "✔︎".light_green(),
        ))
        .machine(&alias)
        .json(serde_json::json!({ "tcp-outlet": { "alias": alias, "node": node } }))
        .write_line()?;
    Ok(())
}

/// Construct a request to delete a tcp outlet
fn make_api_request<'a>(cmd: DeleteCommand) -> Result<RequestBuilder<'a>> {
    let alias = cmd.alias;
    let request = Request::delete(format!("/node/outlet/{alias}"));
    Ok(request)
}
