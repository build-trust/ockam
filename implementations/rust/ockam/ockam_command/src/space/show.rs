use clap::Args;
use console::Term;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::nodes::InMemoryNode;
use ockam_core::AsyncTryClone;

use crate::output::Output;
use crate::terminal::tui::ShowCommandTui;
use crate::terminal::PluralTerm;
use crate::util::api::CloudOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Terminal, TerminalStream};

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
    pub cloud_opts: CloudOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show space".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ShowTui::run(
            ctx.async_try_clone().await.into_diagnostic()?,
            opts,
            self.clone(),
        )
        .await
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    space_name: Option<String>,
    node: InMemoryNode,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(&ctx, &opts.state).await?;
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
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Space;

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.space_name.as_deref()
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
            .get_spaces(&self.ctx)
            .await?
            .iter()
            .map(|s| s.space_name())
            .collect())
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let space = self.node.get_space_by_name(&self.ctx, item_name).await?;
        self.terminal()
            .stdout()
            .plain(space.output()?)
            .json(serde_json::to_string(&space).into_diagnostic()?)
            .machine(&space.name)
            .write_line()?;
        Ok(())
    }

    async fn show_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let spaces: Vec<Space> = self.node.get_spaces(&self.ctx).await?;
        let filtered: Vec<Space> = spaces
            .into_iter()
            .filter(|s| items_names.contains(&s.name))
            .collect();
        let plain = self
            .terminal()
            .build_list(&filtered, "Spaces", "No Spaces found")?;
        let json = serde_json::to_string(&filtered).into_diagnostic()?;
        self.terminal()
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
