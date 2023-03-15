use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::{CommandGlobalOpts, Result};
use clap::Args;
use ockam::Context;
use ockam_abac::Resource;
use ockam_api::nodes::models::policy::PolicyList;
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: String,

    #[arg(short, long)]
    resource: Resource,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(mut ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(ctx: &mut Context, opts: CommandGlobalOpts, cmd: ListCommand) -> Result<()> {
    let resource = cmd.resource;
    let node = extract_address_value(&cmd.at)?;
    let req = Request::get(format!("/policy/{resource}"));
    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    rpc.request(req).await?;
    let pol: PolicyList = rpc.parse_response()?;
    for (a, e) in pol.expressions() {
        println!("{resource}/{a}: {e}")
    }
    Ok(())
}
