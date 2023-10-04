use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::{Request, RequestBuilder};

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{docs, CommandGlobalOpts};
use crate::{fmt_ok, Result};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP inlet's details
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Name of the inlet
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which the inlet was started
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
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    let mut rpc = Rpc::background(&ctx, &opts, &node_name).await?;
    let inlet_status: InletStatus = rpc.ask(make_api_request(cmd)?).await?;

    let json = serde_json::to_string(&inlet_status).into_diagnostic()?;
    let InletStatus {
        alias,
        bind_addr,
        outlet_route,
        ..
    } = inlet_status;
    let plain = formatdoc! {r#"
        Inlet:
          Alias: {alias}
          TCP Address: {bind_addr}
          To Outlet Address: {outlet_route}
    "#};
    let machine = bind_addr;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("{}", plain))
        .machine(machine)
        .json(json)
        .write_line()?;
    Ok(())
}

/// Construct a request to show a tcp inlet
fn make_api_request(cmd: ShowCommand) -> Result<RequestBuilder> {
    let alias = cmd.alias;
    let request = Request::get(format!("/node/inlet/{alias}"));
    Ok(request)
}
