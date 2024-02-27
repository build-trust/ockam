use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::Address;

use crate::output::Output;
use crate::util::async_cmd;
use crate::{docs, util::api, CommandGlobalOpts};

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
    at: Option<String>,

    /// Channel address
    #[arg(display_order = 800)]
    address: Address,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "secure-channel show".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.at).await?;

        let address = &self.address;
        let response: ShowSecureChannelResponse =
            node.ask(ctx, api::show_secure_channel(address)).await?;
        opts.terminal
            .stdout()
            .plain(response.output()?)
            .json(serde_json::to_string_pretty(&response).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
