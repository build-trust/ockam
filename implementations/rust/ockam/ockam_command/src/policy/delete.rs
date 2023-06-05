use crate::policy::policy_path;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{CommandGlobalOpts, Result};
use clap::Args;
use ockam::Context;
use ockam_abac::{Action, Resource};
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: String,

    #[arg(short, long)]
    resource: Resource,

    #[arg(short, long)]
    action: Action,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(ctx: &mut Context, opts: CommandGlobalOpts, cmd: DeleteCommand) -> Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let req = Request::delete(policy_path(&cmd.resource, &cmd.action));
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    rpc.request(req).await?;
    rpc.is_ok()?;
    Ok(())
}
