use clap::Args;
use miette::miette;

use ockam::Context;
use ockam_core::Address;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::secure_channel::listener::utils::SecureChannelListenerNodeOpts;
use crate::util::{api, exitcode, extract_address_value, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channel Listener
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Address of the channel listener
    address: Address,

    #[command(flatten)]
    node_opts: SecureChannelListenerNodeOpts,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at);
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
    let at = get_node_name(&opts.state, &cmd.node_opts.at);
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
            miette!("An error occurred while retrieving secure channel listener").context(e),
        )),
    }
}
