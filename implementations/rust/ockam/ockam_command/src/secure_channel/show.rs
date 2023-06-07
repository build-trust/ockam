use crate::{
    docs,
    util::{api, extract_address_value, node_rpc, Rpc},
    CommandGlobalOpts, Result,
};
use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_core::Address;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Node at which the secure channel was initiated
    #[arg(value_name = "NODE_NAME", long, display_order = 800)]
    at: String,

    /// Channel address
    #[arg(display_order = 800)]
    address: Address,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    // Read the `at` argument and return node name
    fn parse_at_node(&self) -> String {
        extract_address_value(&self.at).unwrap_or_else(|_| "".to_string())
    }
}

async fn rpc(ctx: Context, (options, command): (CommandGlobalOpts, ShowCommand)) -> Result<()> {
    let at = &command.parse_at_node();
    let address = &command.address;

    let mut rpc = Rpc::background(&ctx, &options, at)?;
    let request = api::show_secure_channel(address);
    rpc.request(request).await?;
    let response = rpc.parse_response::<ShowSecureChannelResponse>()?;

    rpc.print_response(response)?;

    Ok(())
}
