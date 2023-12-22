use clap::Args;

use ockam::Context;
use ockam_abac::{Action, Expr, Policy, Resource};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::node::util::initialize_default_node;
use crate::policy::policy_path;
use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,

    #[arg(short, long)]
    resource: Resource,

    #[arg(short, long, default_value = "handle_message")]
    action: Action,

    #[arg(short, long)]
    expression: Expr,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    initialize_default_node(ctx, &opts).await?;
    let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
    let bdy = Policy::new(cmd.expression);
    let req = Request::post(policy_path(&cmd.resource, &cmd.action)).body(bdy);
    node.tell(ctx, req).await?;
    Ok(())
}
