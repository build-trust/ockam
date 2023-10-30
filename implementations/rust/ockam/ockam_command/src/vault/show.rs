use std::fmt::Write;

use clap::Args;
use miette::IntoDiagnostic;

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::VaultConfig;

use crate::util::local_cmd;
use crate::vault::list::VaultListOutput;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a vault
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name of the vault
    pub name: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ShowCommand) -> miette::Result<()> {
    // when interactive prompt is not required
    if cmd.name.is_some() || !opts.terminal.can_ask_for_user_input() {
        let name = cmd
            .name
            .unwrap_or(opts.state.vaults.default()?.name().to_string());
        let state = opts.state.vaults.get(name)?;

        let json = serde_json::to_string_pretty(&state).into_diagnostic()?;

        let plain = {
            let mut buf = String::new();

            writeln!(buf, "Vault:").into_diagnostic()?;
            for line in state.to_string().lines() {
                writeln!(buf, "{:2}{}", "", line).into_diagnostic()?;
            }
            buf
        };

        opts.terminal
            .stdout()
            .json(json)
            .plain(plain)
            .write_line()?;
    } else {
        let vault_names: Vec<String> = opts.state.vaults.list_items_names()?;

        match vault_names.len() {
            0 => {
                opts.terminal
                    .stdout()
                    .plain(
                        "There are no vaults to show, use `ockam vault create` to create a new vault",
                    )
                    .write_line()?;
            }

            1 => {
                let name = vault_names[0];
                let state = opts.state.vaults.get(name)?;

                let json = serde_json::to_string_pretty(&state).into_diagnostic()?;

                let plain = {
                    let mut buf = String::new();

                    writeln!(buf, "Vault:").into_diagnostic()?;
                    for line in state.to_string().lines() {
                        writeln!(buf, "{:2}{}", "", line).into_diagnostic()?;
                    }
                    buf
                };

                opts.terminal
                    .stdout()
                    .json(json)
                    .plain(plain)
                    .write_line()?;
            }

            _ => {
                let selected_names = opts.terminal.select_multiple(
                    "Select the vaults which you want to display".to_string(),
                    vault_names,
                );

                if selected_names.is_empty() {
                    opts.terminal
                        .stdout()
                        .plain("No vaults selected, use <space> to select vaults")
                        .write_line()?;
                } else {
                    let mut output: Vec<VaultListOutput> = Vec::new();

                    for name in selected_names {
                        let state = opts.state.vaults.get(&name)?;
                        let config = VaultConfig::new(state.is_aws())?;
                        let is_default = opts.state.vaults.is_default(&name)?;
                        let vault = VaultListOutput::new(name, config, is_default);
                        output.push(vault);
                    }

                    // same output as vault list
                    let plain = opts.terminal.build_list(
                        &output,
                        "Vaults",
                        "No vaults found on this system.",
                    )?;

                    let json = serde_json::to_string_pretty(&output).into_diagnostic()?;

                    opts.terminal
                        .clone()
                        .stdout()
                        .plain(plain)
                        .json(json)
                        .write_line()?;
                }
            }
        }
    }

    Ok(())
}
