use crate::help;
use crate::node::NodeOpts;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::identity::CreateIdentityResponse;
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct CreateCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (options, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let mut rpc = Rpc::background(&ctx, &options, &cmd.node_opts.api_node)?;
    let request = Request::post("/node/identity");
    rpc.request(request).await?;
    let res = rpc.parse_response::<CreateIdentityResponse>()?;
    println!("Identity {} created!", res.identity_id);
    Ok(())
}
