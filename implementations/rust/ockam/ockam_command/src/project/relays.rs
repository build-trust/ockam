use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::{IdentityOpts, TimeoutArg, TrustOpts};
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::fmt_log;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use std::sync::Arc;
use std::time::Duration;

const HELP_DETAIL: &str = "";

/// List relays on the project node
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ListRelaysCommand {
    #[command(flatten)]
    pub timeout: TimeoutArg,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    trust_opts: TrustOpts,
}

#[derive(Clone)]
struct ListRelayNodeCommand {
    opts: CommandGlobalOpts,
    command: ListRelaysCommand,
}

impl ListRelayNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ListRelaysCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ListRelayNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.trust_opts.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    fn timeout(&self) -> Option<Duration> {
        Some(self.command.timeout.timeout)
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        self.opts
            .terminal
            .write_line(fmt_log!("Listing relays at project node ...\n"))?;
        let project = node.get_project_by_name_or_default(&None).await?;
        let client = node
            .create_project_client(
                &project.project_identifier().unwrap(),
                project.project_multiaddr().unwrap(),
                None,
                ockam_api::orchestrator::CredentialsEnabled::On,
            )
            .await?;
        let relays = client.list_relays(ctx).await?;

        let plain = &self.opts.terminal.build_list(&relays, "No relays found")?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json_obj(relays)?
            .write_line()?;

        Ok(())
    }
}

#[async_trait]
impl Command for ListRelaysCommand {
    const NAME: &'static str = "project relays";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ListRelayNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
