use crate::tcp::util::alias_parser;
use crate::tui::{PluralTerm, ShowCommandTui};
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use console::Term;
use miette::{miette, IntoDiagnostic};
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::api::Request;
use ockam_core::TryClone;

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP Inlet's details
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    /// Name of the inlet
    #[arg(display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Show Inlet at the specified node. If you don't provide it, the default node will be used
    #[arg(long, display_order = 903, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "tcp-inlet show";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx.try_clone().into_diagnostic()?, opts, self.clone()).await?)
    }
}

pub struct ShowTui {
    pub ctx: Context,
    pub opts: CommandGlobalOpts,
    pub cmd: ShowCommand,
    pub node: BackgroundNodeClient,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        mut cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(&ctx, &opts.state, &cmd.at).await?;
        cmd.at = Some(node.node_name());

        let tui = Self {
            ctx,
            opts,
            cmd,
            node,
        };

        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::TcpInlet;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.cmd.alias.clone()
    }

    fn node_name(&self) -> Option<&str> {
        self.cmd.at.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd
            .alias
            .clone()
            .ok_or(miette!("No TCP Inlet alias provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        let inlets: Vec<InletStatus> = self
            .node
            .ask(&self.ctx, Request::get("/node/inlet"))
            .await?;
        let items_names: Vec<String> = inlets
            .into_iter()
            .map(|inlet| inlet.alias.to_string())
            .collect();
        Ok(items_names)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let inlet_status: InletStatus = self
            .node
            .ask(&self.ctx, Request::get(format!("/node/inlet/{item_name}")))
            .await?;
        self.terminal()
            .stdout()
            .plain(inlet_status.item()?)
            .json_obj(inlet_status)?
            .write_line()?;
        Ok(())
    }
}
