use crate::node::{get_node_name, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::CommandGlobalOpts;
use crate::Result;
use clap::Args;
use ockam::{route, Context};
use ockam_api::error::ApiError;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::route_to_multiaddr;
use ockam_core::api::{Request, RequestBuilder};

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command()]
pub struct ShowCommand {
    /// Name assigned to outlet that will be shown
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node from the outlet that is to be shown. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&options.state, cmd.node_opts.api_node.clone())?;
    let node_name = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &options, &node_name)?;
    rpc.request(make_api_request(cmd)?).await?;
    rpc.is_ok()?;

    let outlet_to_show = rpc.parse_response::<OutletStatus>()?;

    println!("Outlet:");
    println!("  Alias: {}", outlet_to_show.alias);
    let addr = route_to_multiaddr(&route![outlet_to_show.worker_addr.to_string()])
        .ok_or_else(|| ApiError::generic("Invalid Outlet Address"))?;
    println!("  From Outlet: {addr}");
    println!("  To TCP: {}", outlet_to_show.tcp_addr);
    Ok(())
}

/// Construct a request to show a tcp outlet
fn make_api_request<'a>(cmd: ShowCommand) -> Result<RequestBuilder<'a>> {
    let alias = cmd.alias;
    let request = Request::get(format!("/node/outlet/{alias}"));
    Ok(request)
}
