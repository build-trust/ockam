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

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "lease show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let cmd = self.parse_args(&opts).await?;

        let node = InMemoryNode::start_with_identity_and_project_name(
            ctx,
            &opts.state,
            cmd.identity_opts.identity_name.clone(),
            cmd.trust_opts.project_name.clone(),
        )
        .await?
        .with_timeout(cmd.timeout.timeout);

        opts.terminal
            .write_line(fmt_log!("Retrieving influxdb token...\n"))?;

        let (at, _meta) = clean_nodes_multiaddr(&cmd.at, &opts.state).await?;
        let res = node.get_token(ctx, &at, &cmd.token_id).await?;

        opts.terminal
            .stdout()
            .machine(res.token.to_string())
            .plain(res.item()?)
            .json_obj(res)?
            .write_line()?;

        Ok(())
    }
}

impl ShowCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> crate::Result<Self> {
        self.at = super::resolve_at_arg(&self.at, &opts.state).await?;
        Ok(self)
    }
}
