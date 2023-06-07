use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::Result;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use ockam::{Context, Route};
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::route_to_multiaddr;
use ockam_core::api::{Request, RequestBuilder};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Delete a TCP Inlet
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Name assigned to inlet that will be shown
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node from the inlet that is to be shown. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = extract_address_value(&node_name)?;
    let mut rpc = Rpc::background(&ctx, &opts, &node_name)?;
    rpc.request(make_api_request(cmd)?).await?;
    rpc.is_ok()?;

    let inlet_to_show = rpc.parse_response::<InletStatus>()?;

    println!("Inlet:");
    println!("  Alias: {}", inlet_to_show.alias);
    println!("  TCP Address: {}", inlet_to_show.bind_addr);
    if let Some(r) = Route::parse(inlet_to_show.outlet_route.as_ref()) {
        if let Some(ma) = route_to_multiaddr(&r) {
            println!("  To Outlet Address: {ma}");
        }
    }
    Ok(())
}

/// Construct a request to show a tcp inlet
fn make_api_request<'a>(cmd: ShowCommand) -> Result<RequestBuilder<'a>> {
    let alias = cmd.alias;
    let request = Request::get(format!("/node/inlet/{alias}"));
    Ok(request)
}
