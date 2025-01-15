use crate::shared_args::{IdentityOpts, TimeoutArg, TrustOpts};
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::influxdb::lease_issuer::InfluxDBTokenLessorNodeServiceTrait;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_log, fmt_ok};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

const HELP_DETAIL: &str = "";

/// Create a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct CreateCommand {
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
impl Command for CreateCommand {
    const NAME: &'static str = "lease create";

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
            .write_line(fmt_log!("Creating influxdb token...\n"))?;

        let res = node.create_token(ctx, &cmd.at).await?;

        let plain = fmt_ok!("A token with id {}\n", color_primary(&res.id))
            + &fmt_log!(
                "was issued for the identifier {}\n",
                color_primary(&res.issued_for)
            )
            + &fmt_log!("and will expire at {}", color_primary(res.expires_at()?));

        opts.terminal
            .stdout()
            .machine(&res.token)
            .plain(plain)
            .json_obj(res)?
            .write_line()?;

        Ok(())
    }
}

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> crate::Result<Self> {
        self.at = super::resolve_at_arg(&self.at, &opts.state).await?;
        Ok(self)
    }
}
