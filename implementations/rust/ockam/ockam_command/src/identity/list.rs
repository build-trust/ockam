use std::fmt::Write;

use clap::Args;
use colorful::Colorful;
use ockam_api::colors::OckamColor;
use ockam_api::output::Output;
use serde::Serialize;
use serde_json::json;

use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List identities
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {}

impl ListCommand {
    pub fn name(&self) -> String {
        "identity list".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        let mut identities_list: Vec<IdentityListOutput> = Vec::new();

        let identities = opts.state.get_named_identities().await?;
        for identity in identities.iter() {
            let identity_output = IdentityListOutput::new(
                identity.name(),
                identity.identifier().to_string(),
                identity.is_default(),
            );
            identities_list.push(identity_output);
        }

        let list = opts
            .terminal
            .build_list(&identities_list, "No identities found on this system.")?;

        opts.terminal
            .stdout()
            .plain(list)
            .json(json!(&identities))
            .write_line()?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct IdentityListOutput {
    pub name: String,
    pub identifier: String,
    pub is_default: bool,
}

impl IdentityListOutput {
    pub fn new(name: String, identifier: String, is_default: bool) -> Self {
        Self {
            name,
            identifier,
            is_default,
        }
    }
}

impl Output for IdentityListOutput {
    fn item(&self) -> ockam_api::Result<String> {
        let default = if self.is_default { "(default)" } else { "" };
        let mut output = String::new();
        writeln!(
            output,
            "Identity {name} {default}",
            name = self
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "{identifier}",
            identifier = self
                .identifier
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        Ok(output)
    }
}
