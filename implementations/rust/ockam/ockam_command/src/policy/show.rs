use crate::policy::policy_path;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{CommandGlobalOpts, Result};
use clap::Args;
use ockam::Context;
use ockam_abac::{Action, Resource};
use ockam_api::nodes::models::policy::Policy;
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
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

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(ctx: &mut Context, opts: CommandGlobalOpts, cmd: ShowCommand) -> Result<()> {
    let node = extract_address_value(&cmd.at)?;
    let req = Request::get(policy_path(&cmd.resource, &cmd.action));
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    rpc.request(req).await?;
    let pol: Policy = rpc.parse_response()?;
    println!("{}", pol.expression());
    Ok(())
}
