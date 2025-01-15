use async_trait::async_trait;
use clap::Args;
use console::Term;

use crate::{docs, Command, CommandGlobalOpts};
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;
use ockam_api::terminal::{Terminal, TerminalStream};

use crate::shared_args::IdentityOpts;
use crate::terminal::tui::ShowCommandTui;
use crate::tui::PluralTerm;
use ockam_api::output::Output;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a space
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the space
    #[arg(display_order = 1001)]
    pub name: Option<String>,

    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "space show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        Ok(ShowTui::run(ctx, opts, self).await?)
    }
}

pub struct ShowTui<'a> {
    ctx: &'a Context,
    opts: CommandGlobalOpts,
    space_name: Option<String>,
    node: InMemoryNode,
}

impl<'a> ShowTui<'a> {
    pub async fn run(
        ctx: &'a Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let tui = Self {
            ctx,
            opts,
            space_name: cmd.name,
            node,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl<'a> ShowCommandTui for ShowTui<'a> {
    const ITEM_NAME: PluralTerm = PluralTerm::Space;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.space_name.clone()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let space_name = match &self.space_name {
            None => self.opts.state.get_default_space().await?.space_name(),
            Some(n) => n.to_string(),
        };
        Ok(space_name)
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self
            .node
            .get_spaces(self.ctx)
            .await?
            .iter()
            .map(|s| s.space_name())
            .collect())
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let space = self.node.get_space_by_name(self.ctx, item_name).await?;
        self.terminal()
            .stdout()
            .plain(space.item()?)
            .json_obj(&space)?
            .machine(&space.name)
            .write_line()?;
        Ok(())
    }
}
