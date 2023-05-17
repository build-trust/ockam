use crate::util::output::Output;
use crate::util::{exitcode, node_rpc};
use crate::{docs, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List identities
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[arg(short, long)]
    full: bool,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(Self::run_impl, (options, self))
    }

    async fn run_impl(
        _ctx: Context,
        options: (CommandGlobalOpts, ListCommand),
    ) -> crate::Result<()> {
        let (opts, cmd) = options;
        let idts = opts.state.identities.list()?;
        if idts.is_empty() {
            return Err(crate::Error::new(
                exitcode::IOERR,
                anyhow!("No identities registered on this system!"),
            ));
        }
        for (idx, identity) in idts.iter().enumerate() {
            let state = opts.state.identities.get(identity.name())?;
            let default = if opts.state.identities.default()?.name() == identity.name() {
                " (default)"
            } else {
                ""
            };
            println!("Identity[{idx}]:");
            println!("{:2}Name: {}{}", "", &identity.name(), default);
            if cmd.full {
                let identifier = state.config().identifier();
                let identity = opts
                    .state
                    .identities
                    .identities_repository()
                    .await?
                    .get_identity(&identifier)
                    .await?;
                let identity = identity.export()?;
                let output = LongIdentityResponse::new(identity);
                println!("{:2}{}", "", &output.output()?);
            } else {
                let output = ShortIdentityResponse::new(state.config().identifier().to_string());
                println!("{:2}Identifier: {}", "", &output.output()?);
            };
            if idx < idts.len() - 1 {
                println!();
            }
        }
        Ok(())
    }
}
