use clap::Args;
use colorful::Colorful;
use console::Term;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use ockam_api::colors::OckamColor;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::ConnectionStatus;
use ockam_core::TryClone;
use serde::Serialize;

use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a Relay given its name
#[derive(Clone, Debug, Args)]
#[command(
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name assigned to the Relay
    relay_name: Option<String>,

    /// Node which the relay belongs to
    #[arg(long, value_name = "NODE", value_parser = extract_address_value)]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }
    pub fn name(&self) -> String {
        "relay show".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(ctx.try_clone().into_diagnostic()?, opts, self.clone()).await
    }
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

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.relay_name.clone()
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
            .ask(&self.ctx, Request::get("/node/relay"))
            .await?;
        Ok(relays.into_iter().map(|i| i.name().to_string()).collect())
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let relay: RelayInfo = self
            .node
            .ask(&self.ctx, Request::get(format!("/node/relay/{item_name}")))
            .await?;
        let relay = RelayShowOutput::from(relay);
        self.terminal()
            .stdout()
            .plain(relay.item()?)
            .machine(item_name)
            .json(serde_json::to_string(&relay).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

#[derive(Serialize)]
struct RelayShowOutput {
    pub name: String,
    pub destination: MultiAddr,
    pub connection_status: ConnectionStatus,
    pub relay_route: Option<String>,
    pub remote_address: Option<MultiAddr>,
    pub worker_address: Option<MultiAddr>,
}

impl From<RelayInfo> for RelayShowOutput {
    fn from(r: RelayInfo) -> Self {
        Self {
            name: r.name().to_string(),
            destination: r.destination_address().clone(),
            connection_status: r.connection_status(),
            relay_route: r.forwarding_route().clone(),
            remote_address: r.remote_address_ma().into_diagnostic().unwrap(),
            worker_address: r.worker_address_ma().into_diagnostic().unwrap(),
        }
    }
}

impl Output for RelayShowOutput {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(formatdoc!(
            r#"
        Relay:
            Name: {alias}
            Destination: {destination_address}
            Status: {connection_status}
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}
        "#,
            alias = self.name,
            connection_status = self.connection_status.to_string(),
            destination_address = self.destination.to_string(),
            route = self.relay_route.as_deref().unwrap_or("N/A"),
            remote_addr = self
                .remote_address
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("N/A".into()),
            worker_addr = self
                .worker_address
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("N/A".into()),
        ))
    }

    fn as_list_item(&self) -> ockam_api::Result<String> {
        Ok(formatdoc!(
            r#"
            Name: {alias}
            Status: {connection_status}
            Remote Address: {remote_address}"#,
            alias = self
                .name
                .as_str()
                .color(OckamColor::PrimaryResource.color()),
            connection_status = self.connection_status.to_string(),
            remote_address = self
                .remote_address
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("N/A".into())
                .color(OckamColor::PrimaryResource.color()),
        ))
    }
}
