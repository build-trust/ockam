use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/default/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/default/after_long_help.txt");

/// Change the default identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct DefaultCommand {
    /// Name of the identity to be set as default
    name: Option<String>,
}

impl DefaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DefaultCommand),
) -> miette::Result<()> {
    match cmd.name {
        Some(name) => {
            if opts.state.is_default_identity_by_name(&name).await? {
                Err(miette!(
                    "The identity named '{}' is already the default",
                    &name
                ))?
            } else {
                opts.state.set_as_default_identity(&name).await?;
                opts.terminal
                    .stdout()
                    .plain(fmt_ok!("The identity named '{}' is now the default", &name))
                    .machine(&name)
                    .write_line()?;
            }
        }
        None => {
            let identity = opts.state.get_or_create_default_named_identity().await?;
            opts.terminal
                .stdout()
                .plain(fmt_ok!(
                    "The name of the default identity is '{}'",
                    identity.name()
                ))
                .write_line()?;
        }
    };

    Ok(())
}
