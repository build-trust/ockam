use clap::Args;
use console::Term;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use serde::Serialize;

use crate::output::Output;
use crate::relay::util::relay_name_parser;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Terminal, TerminalStream};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a Relay given its name
#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name assigned to the Relay, prefixed with 'forward_to_'. Example: 'forward_to_myrelay'
    #[arg(value_parser = relay_name_parser)]
    relay_name: Option<String>,

    /// Node which the relay belongs to
    #[arg(long, value_name = "NODE", value_parser = extract_address_value)]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    ShowTui::run(ctx, opts, cmd).await
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: ShowCommand,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Relay;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.cmd.relay_name.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd
            .relay_name
            .clone()
            .ok_or(miette!("No relay name provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let relays: Vec<RelayInfo> = self
            .node
            .ask(&self.ctx, Request::get("/node/forwarder"))
            .await?;
        let names = relays
            .into_iter()
            .map(|i| i.remote_address().to_string())
            .collect();
        Ok(names)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let relay: RelayInfo = self
            .node
            .ask(
                &self.ctx,
                Request::get(format!("/node/forwarder/{item_name}")),
            )
            .await?;
        let relay = RelayShowOutput::from(relay);
        self.terminal()
            .stdout()
            .plain(relay.output()?)
            .machine(item_name)
            .json(serde_json::to_string(&relay).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }

    async fn show_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let relays: Vec<RelayInfo> = self
            .node
            .ask(&self.ctx, Request::get("/node/forwarder"))
            .await?;
        let relays = relays
            .into_iter()
            .filter(|it| items_names.contains(&it.remote_address().to_string()))
            .map(RelayShowOutput::from)
            .collect::<Vec<RelayShowOutput>>();
        let node_name = self.node.node_name();
        let plain = self.terminal().build_list(
            &relays,
            &format!("Relays on Node {node_name}"),
            &format!("No Relays found on Node {node_name}."),
        )?;
        let json = serde_json::to_string(&relays).into_diagnostic()?;
        self.terminal()
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}

#[derive(Serialize)]
struct RelayShowOutput {
    pub relay_route: String,
    pub remote_address: MultiAddr,
    pub worker_address: MultiAddr,
}

impl From<RelayInfo> for RelayShowOutput {
    fn from(r: RelayInfo) -> Self {
        Self {
            relay_route: r.forwarding_route().to_string(),
            remote_address: r.remote_address_ma().into_diagnostic().unwrap(),
            worker_address: r.worker_address_ma().into_diagnostic().unwrap(),
        }
    }
}

impl Output for RelayShowOutput {
    fn output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
        Relay:
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}
        "#,
            route = self.relay_route,
            remote_addr = self.remote_address,
            worker_addr = self.worker_address,
        ))
    }

    fn list_output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}"#,
            route = self.relay_route,
            remote_addr = self.remote_address,
            worker_addr = self.worker_address,
        ))
    }
}
