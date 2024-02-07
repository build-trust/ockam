use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::service::portals::Inlets;
use ockam_api::nodes::BackgroundNodeClient;

use crate::node::NodeOpts;
use crate::tcp::util::alias_parser;
use crate::util::async_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP Inlet's details
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show tcp inlet".into()
    }

    pub async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let inlet_status = node
            .show_inlet(ctx, &self.alias)
            .await?
            .success()
            .into_diagnostic()?;

        let json = serde_json::to_string(&inlet_status).into_diagnostic()?;
        let InletStatus {
            alias,
            bind_addr,
            outlet_route,
            status,
            outlet_addr,
            ..
        } = inlet_status;

        let outlet_route = outlet_route.unwrap_or("N/A".to_string());
        let plain = formatdoc! {r#"
        Inlet:
          Alias: {alias}
          Status: {status}
          TCP Address: {bind_addr}
          Outlet Route: {outlet_route}
          Outlet Destination: {outlet_addr}
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
}
