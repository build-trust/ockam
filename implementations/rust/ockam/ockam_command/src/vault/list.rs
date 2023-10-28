use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use std::fmt::Write;

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::StateItemTrait;
use ockam_api::cli_state::VaultConfig;

use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::local_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List vaults
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

#[derive(serde::Serialize)]
pub struct VaultListOutput {
    name: String,
    #[serde(flatten)]
    config: VaultConfig,
    is_default: bool,
}

impl VaultListOutput {
    pub fn new(name: String, config: VaultConfig, is_default: bool) -> Self {
        Self {
            name,
            config,
            is_default,
        }
    }
}

impl Output for VaultListOutput {
    fn output(&self) -> crate::error::Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Vault {} {}",
            self.name
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            if self.is_default { "(default)" } else { "" }
        )?;
        write!(
            output,
            "Type {}",
            match self.config.is_aws() {
                true => "AWS KMS",
                false => "OCKAM",
            }
            .to_string()
            .color(OckamColor::PrimaryResource.color())
        )?;
        Ok(output)
    }
}

fn run_impl(opts: CommandGlobalOpts) -> miette::Result<()> {
    let vaults = opts.state.vaults.list()?;

    let output = vaults
        .iter()
        .map(|v| {
            VaultListOutput::new(
                v.name().to_string(),
                v.config().clone(),
                opts.state.vaults.is_default(v.name()).unwrap_or(false),
            )
        })
        .collect::<Vec<VaultListOutput>>();

    let plain = opts
        .terminal
        .build_list(&output, "Vaults", "No vaults found on this system.")?;

    let json = serde_json::to_string_pretty(&output).into_diagnostic()?;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}
