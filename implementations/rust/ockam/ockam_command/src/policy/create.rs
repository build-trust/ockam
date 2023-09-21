use clap::Args;

use ockam::Context;
use ockam_abac::{Action, Expr, Resource};
use ockam_api::nodes::models::policy::Policy;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::get_node_name;
use crate::policy::policy_path;
use crate::util::{node_rpc, parse_node_name};
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
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;
    let bdy = Policy::new(cmd.expression);
    let req = Request::post(policy_path(&cmd.resource, &cmd.action)).body(bdy);
    let node = BackgroundNode::create(ctx, &opts.state, &node_name).await?;
    node.tell(ctx, req).await?;
    Ok(())
}
