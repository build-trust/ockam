use clap::Args;
use console::Term;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait, StateItemTrait};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::cloud::Controller;
use ockam_api::nodes::InMemoryNode;

use crate::output::Output;
use crate::terminal::tui::ShowCommandTui;
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
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
    space_name: Option<String>,
    controller: Controller,
}

impl ShowTui {
    pub async fn run(
        ctx: Context,
        opts: CommandGlobalOpts,
        cmd: ShowCommand,
    ) -> miette::Result<()> {
        let node = InMemoryNode::start(&ctx, &opts.state).await?;
        let controller = node.create_controller().await?;
        let tui = Self {
            ctx,
            opts,
            space_name: cmd.name,
            controller,
        };
        tui.show().await
    }
}

#[ockam_core::async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: &'static str = "space";

    fn cmd_arg_item_name(&self) -> Option<&str> {
        self.space_name.as_deref()
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        let space_name = match &self.space_name {
            None => self.opts.state.spaces.default()?.name().to_string(),
            Some(n) => n.to_string(),
        };
        Ok(space_name)
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        Ok(self.opts.state.spaces.list_items_names()?)
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let id = self.opts.state.spaces.get(item_name)?.config().id.clone();
        let space = self.controller.get_space(&self.ctx, id).await?;
        self.opts
            .state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
        self.terminal()
            .stdout()
            .plain(space.output()?)
            .json(serde_json::to_string(&space).into_diagnostic()?)
            .machine(&space.name)
            .write_line()?;
        Ok(())
    }

    async fn show_multiple(&self, items_names: Vec<String>) -> miette::Result<()> {
        let spaces: Vec<Space> = self.controller.list_spaces(&self.ctx).await?;
        for space in &spaces {
            self.opts
                .state
                .spaces
                .overwrite(&space.name, SpaceConfig::from(space))?;
        }
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
