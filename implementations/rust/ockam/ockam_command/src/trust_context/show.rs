use clap::Args;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a trust context
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the trust context
    pub name: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let trust_context = opts.state.get_trust_context_or_default(&cmd.name).await?;
    let plain_output = {
        let mut output = "Trust context:".to_string();
        for line in trust_context.to_string().lines() {
            output.push_str(&format!("{:2}{}\n", "", line));
        }
        output
    };
    opts.terminal.stdout().plain(plain_output).write_line()?;
    Ok(())
}
