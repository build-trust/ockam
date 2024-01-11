use clap::Args;
use miette::miette;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List trust contexts
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand;

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, opts);
    }
}

async fn run_impl(_ctx: Context, opts: CommandGlobalOpts) -> miette::Result<()> {
    let trust_contexts = opts.state.get_trust_contexts().await?;
    if trust_contexts.is_empty() {
        return Err(miette!("No trust contexts registered on this system!"));
    }
    let plain_output = {
        let mut output = String::new();
        for (idx, tc) in trust_contexts.iter().enumerate() {
            output.push_str(&format!("Trust context[{idx}]:"));
            for line in tc.to_string().lines() {
                output.push_str(&format!("{:2}{}\n", "", line));
            }
            output.push('\n');
        }
        output
    };
    opts.terminal.stdout().plain(plain_output).write_line()?;
    Ok(())
}
