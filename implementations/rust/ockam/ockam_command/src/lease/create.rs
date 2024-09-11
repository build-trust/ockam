use crate::shared_args::{IdentityOpts, TimeoutArg, TrustOpts};
use crate::util::clean_nodes_multiaddr;
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam_api::fmt_log;
use ockam_api::influxdb::token_lessor_node_service::InfluxDbTokenLessorNodeServiceTrait;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

const HELP_DETAIL: &str = "";

/// Create a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct CreateCommand {
    /// The route to the node that will be used to create the token
    #[arg(long, value_name = "ROUTE")]
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

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let node = InMemoryNode::start_with_identity_and_project_name(
            ctx,
            &opts.state,
            self.identity_opts.identity_name.clone(),
            self.trust_opts.project_name.clone(),
        )
        .await?
        .with_timeout(self.timeout.timeout);

        opts.terminal
            .write_line(&fmt_log!("Creating influxdb token...\n"))?;

        let (at, _meta) = clean_nodes_multiaddr(&self.at, &opts.state).await?;
        let res = node.create_token(ctx, &at).await?;

        opts.terminal
            .stdout()
            .machine(res.token.to_string())
            .json(serde_json::to_string(&res).into_diagnostic()?)
            .plain(res.item()?)
            .write_line()?;

        Ok(())
    }
}
