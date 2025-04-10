use async_trait::async_trait;
use clap::Args;
use console::Term;
use miette::{miette, IntoDiagnostic};
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::AuthorityNodeClient;
use ockam_api::output::Output;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_core::TryClone;
use std::str::FromStr;
use std::sync::Arc;

use crate::node_command::InMemoryNodeCommand;
use crate::project_member::MemberOutput;
use crate::shared_args::IdentityOpts;
use crate::tui::{PluralTerm, ShowCommandTui};
use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a member from a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
hide = true,
)]
pub struct ShowCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project that the member belongs to
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,

    /// The Identifier of the member to show
    #[arg(value_name = "IDENTIFIER")]
    member: Option<Identifier>,
}

#[derive(Clone)]
struct ShowNodeCommand {
    opts: CommandGlobalOpts,
    command: ShowCommand,
}

impl ShowNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ShowCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ShowNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let tui = ShowTui {
            ctx: node.ctx().try_clone().into_diagnostic()?,
            opts: self.opts.clone(),
            member: self.command.member.clone(),
            client: self.authority_client(node).await?,
        };
        tui.show().await
    }
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "project-member show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ShowNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

pub struct ShowTui {
    ctx: Context,
    opts: CommandGlobalOpts,
    member: Option<Identifier>,
    client: AuthorityNodeClient,
}

#[async_trait]
impl ShowCommandTui for ShowTui {
    const ITEM_NAME: PluralTerm = PluralTerm::Member;

    fn cmd_arg_item_name(&self) -> Option<String> {
        self.member.as_ref().map(|m| m.to_string())
    }

    fn terminal(&self) -> Terminal<TerminalStream<Term>> {
        self.opts.terminal.clone()
    }

    async fn get_arg_item_name_or_default(&self) -> miette::Result<String> {
        self.cmd_arg_item_name()
            .ok_or(miette!("No member provided"))
    }

    async fn list_items_names(&self) -> miette::Result<Vec<String>> {
        self.client.list_member_ids(&self.ctx).await.map(|ids| {
            ids.into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
        })
    }

    async fn show_single(&self, item_name: &str) -> miette::Result<()> {
        let identifier = Identifier::from_str(item_name).into_diagnostic()?;
        let attributes = self.client.show_member(&self.ctx, &identifier).await?;
        let member = MemberOutput::new(identifier, attributes);
        self.terminal()
            .to_stdout()
            .plain(member.item()?)
            .json_obj(&member)?
            .write_line()?;
        Ok(())
    }
}
