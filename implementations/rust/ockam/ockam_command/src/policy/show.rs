use clap::Args;

use ockam::Context;
use ockam_abac::{Action, Policy, Resource};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::policy::policy_path;
use crate::util::async_cmd;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: String,

    #[arg(short, long)]
    resource: Resource,

    #[arg(short, long)]
    action: Action,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "show policy".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create_to_node(ctx, &opts.state, &self.at).await?;
        let req = Request::get(policy_path(&self.resource, &self.action));
        let policy: Policy = node.ask(ctx, req).await?;
        println!("{}", policy.expression());
        Ok(())
    }
}
