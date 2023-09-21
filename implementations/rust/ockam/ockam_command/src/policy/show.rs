use clap::Args;

use ockam::Context;
use ockam_abac::{Action, Resource};
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::policy::Policy;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::policy::policy_path;
use crate::util::node_rpc;
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
    let node_name = extract_address_value(&cmd.at)?;
    let req = Request::get(policy_path(&cmd.resource, &cmd.action));
    let node = BackgroundNode::create(ctx, &opts.state, &node_name).await?;
    let policy: Policy = node.ask(ctx, req).await?;
    println!("{}", policy.expression());
    Ok(())
}
