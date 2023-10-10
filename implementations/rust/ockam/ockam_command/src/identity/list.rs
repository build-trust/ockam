use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

use clap::Args;
use colorful::Colorful;

use ockam_api::cli_state::traits::StateDirTrait;

use ockam_node::Context;
use serde::Serialize;
use serde_json::json;
use std::fmt::Write;
use tokio::sync::Mutex;
use tokio::try_join;

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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(Self::run_impl, (options, self))
    }

    async fn run_impl(
        _ctx: Context,
        options: (CommandGlobalOpts, ListCommand),
    ) -> miette::Result<()> {
        let (opts, _cmd) = options;
        let mut identities: Vec<IdentityListOutput> = Vec::new();

        let idts = opts.state.identities.list()?;
        for identity in idts.iter() {
            let is_finished: Mutex<bool> = Mutex::new(false);

            let send_req = async {
                let i = IdentityListOutput::new(
                    identity.name().to_string(),
                    identity.identifier().to_string(),
                    opts.state.identities.default()?.name() == identity.name(),
                );
                *is_finished.lock().await = true;
                Ok(i)
            };

            let output_messages = vec![format!(
                "Retrieving identity {}...\n",
                &identity.name().color(OckamColor::PrimaryResource.color())
            )];

            let progress_output = opts
                .terminal
                .progress_output(&output_messages, &is_finished);

            let (identity_states, _) = try_join!(send_req, progress_output)?;
            identities.push(identity_states);
        }

        let list = opts.terminal.build_list(
            &identities,
            "Identities",
            "No identities found on this system.",
        )?;

        opts.terminal
            .stdout()
            .plain(list)
            .json(json!({"identities": &identities}))
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
    fn output(&self) -> crate::error::Result<String> {
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
