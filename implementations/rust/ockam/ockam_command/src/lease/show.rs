use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::{IdentityOpts, TimeoutArg, TrustOpts};
use crate::util::clean_nodes_multiaddr;
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::fmt_log;
use ockam_api::influxdb::lease_issuer::InfluxDBTokenLessorNodeServiceTrait;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;
use ockam_multiaddr::MultiAddr;
use std::sync::Arc;
use std::time::Duration;

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ShowCommand {
    /// ID of the token to retrieve
    #[arg(value_name = "TOKEN_ID")]
    pub token_id: String,

    /// The route to the node that will be used to create the token
    #[arg(long, value_name = "ROUTE", default_value_t = super::lease_at_default_value())]
    pub at: MultiAddr,

    #[command(flatten)]
    pub timeout: TimeoutArg,

    #[command(flatten)]
    identity_opts: IdentityOpts,

    #[command(flatten)]
    trust_opts: TrustOpts,
}

impl ShowCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> crate::Result<Self> {
        self.at = super::resolve_at_arg(&self.at, opts.state.clone()).await?;
        Ok(self)
    }
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
        self.command.trust_opts.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    fn timeout(&self) -> Option<Duration> {
        Some(self.command.timeout.timeout)
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let cmd = self.command.clone().parse_args(&self.opts).await?;

        self.opts
            .terminal
            .write_line(fmt_log!("Retrieving influxdb token...\n"))?;

        let (at, _meta) = clean_nodes_multiaddr(&cmd.at, self.opts.state.clone()).await?;
        let res = node.get_token(node.ctx(), &at, &cmd.token_id).await?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .machine(res.token.to_string())
            .plain(res.item()?)
            .json_obj(res)?
            .write_line()?;

        Ok(())
    }
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "lease show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ShowNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
