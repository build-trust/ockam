use crate::node::NodeOpts;
use crate::util::{node_rpc, Rpc};
use crate::CommandGlobalOpts;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::models::identity::{RotateKeyRequest, RotateKeyResponse};
use ockam_core::api::Request;

fn default_key_label() -> String {
    "OCKAM_RK".to_string()
}

#[derive(Clone, Debug, Args)]
pub struct RotateKeyCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
    #[arg(short, long, default_value_t=default_key_label())]
    label: String,
}

impl RotateKeyCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, RotateKeyCommand),
) -> crate::Result<()> {
    let mut rpc = Rpc::background(&ctx, &opts, &cmd.node_opts.api_node)?;
    // XXX Should this be post?
    // XXX Is this the best way to pass arguments?
    let request =
        Request::post("/node/identity/actions/rotate_key").body(RotateKeyRequest::new(cmd.label));
    rpc.request(request).await?;
    rpc.parse_response::<RotateKeyResponse>()?;
    println!("key rotated!");
    Ok(())
}
