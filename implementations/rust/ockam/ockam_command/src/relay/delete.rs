use clap::Args;
use colorful::Colorful;
use console::Term;
use miette::IntoDiagnostic;

use crate::{docs, CommandGlobalOpts};
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{color, fmt_ok};
use ockam_core::api::Request;
use ockam_core::TryClone;

use crate::terminal::tui::DeleteCommandTui;
use crate::tui::PluralTerm;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a Relay
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Name assigned to the Relay
    #[arg(value_name = "RELAY_NAME")]
    relay_name: Option<String>,

    /// Node on which to delete the Relay. If not provided, the default node will be used
    #[arg(global = true, long, value_name = "NODE", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Confirm the deletion without prompting
    #[arg(long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn name(&self) -> String {
        "relay delete".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        DeleteTui::run(ctx.try_clone().into_diagnostic()?, opts, self.clone()).await
    }
}

#[derive(TryClone)]
struct DeleteTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    node: BackgroundNodeClient,
    cmd: DeleteCommand,
}

impl DeleteTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: DeleteCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
        let tui = Self {
            ctx,
            opts,
            node,
            cmd,
        };
        tui.delete().await
    }
}

#[ockam_core::async_trait]
impl DeleteCommandTui for DeleteTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Relay;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.relay_name.clone()
    }

    fn cmd_arg_delete_all(&self) -> bool {
        false
    }

    fn cmd_arg_confirm_deletion(&self) -> bool {
        self.cmd.yes
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let relays: Vec<RelayInfo> = self
            .node
            .ask(&self.ctx, Request::get("/node/relay"))
            .await?;
        Ok(relays.into_iter().map(|i| i.name().to_string()).collect())
    }

    async fn delete_single(&self, relay_name: &str) -> miette::Result<()> {
        let node_name = self.node.node_name();
        self.node
            .tell(
                &self.ctx,
                Request::delete(format!("/node/relay/{relay_name}")),
            )
            .await?;
        self.terminal()
            .stdout()
            .plain(fmt_ok!(
                "Relay with name {} on Node {} has been deleted",
                color!(relay_name, OckamColor::PrimaryResource),
                color!(node_name, OckamColor::PrimaryResource)
            ))
            .write_line()?;
        Ok(())
    }
}
