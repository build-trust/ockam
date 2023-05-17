use anyhow::anyhow;
use clap::Args;

use ockam::Context;
use ockam_core::Address;

use super::common::SecureChannelListenerNodeOpts;
use crate::node::get_node_name;
use crate::util::{api, exitcode, extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// Show Secure Channel Listener
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = docs::after_help(HELP_DETAIL))]
pub struct ShowCommand {
    /// Address of the channel listener
    address: Address,

    #[command(flatten)]
    node_opts: SecureChannelListenerNodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> crate::Result<()> {
    run_impl(&ctx, (opts, cmd)).await
}

async fn run_impl(
    ctx: &Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let at = get_node_name(&opts.state, cmd.node_opts.at.clone())?;
    let node = extract_address_value(&at)?;
    let address = &cmd.address;

    let mut rpc = Rpc::background(ctx, &opts, &node)?;
    let req = api::show_secure_channel_listener(address);
    rpc.request(req).await?;

    match rpc.is_ok() {
        Ok(_) => {
            println!("/service/{}", cmd.address.address());
            Ok(())
        }
        Err(e) => Err(crate::error::Error::new(
            exitcode::UNAVAILABLE,
            anyhow!("An error occurred while retrieving secure channel listener").context(e),
        )),
    }
}
