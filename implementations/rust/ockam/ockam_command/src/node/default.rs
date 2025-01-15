use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam_api::fmt_ok;

use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the node to set as default
    node_name: Option<String>,
}

impl DefaultCommand {
    pub fn name(&self) -> String {
        "node default".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if let Some(node_name) = &self.node_name {
            if opts
                .state
                .get_node(node_name)
                .await
                .ok()
                .map(|n| n.is_default())
                .unwrap_or(false)
            {
                return Err(miette!("The node '{node_name}' is already the default"));
            } else {
                opts.state.set_default_node(node_name).await?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("The node '{node_name}' is now the default"))
                    .machine(node_name)
                    .write_line()?;
            }
        } else {
            let default_node_name = opts.state.get_default_node().await?.name();
            let _ = opts
                .terminal
                .stdout()
                .plain(fmt_ok!("The default node is '{default_node_name}'"))
                .write_line();
        }
        Ok(())
    }
}
