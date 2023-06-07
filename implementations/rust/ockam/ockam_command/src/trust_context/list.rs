use clap::Args;
use miette::miette;
use ockam_api::cli_state::traits::StateDirTrait;

use crate::util::local_cmd;
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
        local_cmd(run_impl(opts));
    }
}

fn run_impl(opts: CommandGlobalOpts) -> miette::Result<()> {
    let states = opts.state.trust_contexts.list()?;
    if states.is_empty() {
        return Err(miette!("No trust contexts registered on this system!"));
    }
    let plain_output = {
        let mut output = String::new();
        for (idx, tc) in states.iter().enumerate() {
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
