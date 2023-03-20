use crate::policy::policy_path;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{CommandGlobalOpts, Result};
use clap::Args;
use ockam::Context;
use ockam_abac::{Action, Expr, Resource};
use ockam_api::nodes::models::policy::Policy;
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: String,

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

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(ctx: &mut Context, opts: CommandGlobalOpts, cmd: CreateCommand) -> Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let bdy = Policy::new(cmd.expression);
    let req = Request::post(policy_path(&cmd.resource, &cmd.action)).body(bdy);
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    rpc.request(req).await?;
    rpc.is_ok()?;
    Ok(())
}
