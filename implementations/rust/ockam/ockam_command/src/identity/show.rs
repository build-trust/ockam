use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::util::output::Output;
use crate::util::{node_rpc, println_output};
use crate::{docs, CommandGlobalOpts, EncodeFormat, Result};
use clap::Args;
use core::fmt::Write;
use miette::IntoDiagnostic;
use ockam::identity::identity::IdentityChangeHistory;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of an identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[arg()]
    name: Option<String>,

    #[arg(short, long)]
    full: bool,

    //TODO: see if it make sense to have a --encoding argument shared across commands.
    //      note the only reason this is here right now is that project.json expect the
    //      authority' identity change history to be in hex format.  This only applies
    //      for `full` (change history) identity.
    #[arg(long, value_enum, requires = "full")]
    encoding: Option<EncodeFormat>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.name);
        node_rpc(Self::run_impl, (opts, self))
    }

    async fn run_impl(
        _ctx: Context,
        options: (CommandGlobalOpts, ShowCommand),
    ) -> miette::Result<()> {
        let (opts, cmd) = options;
        let name = get_identity_name(&opts.state, &cmd.name);
        let state = opts.state.identities.get(&name)?;
        if cmd.full {
            let identifier = state.config().identifier();
            let identity = opts
                .state
                .identities
                .identities_repository()
                .await?
                .get_identity(&identifier)
                .await
                .into_diagnostic()?
                .export()
                .into_diagnostic()?;

            if Some(EncodeFormat::Hex) == cmd.encoding {
                println_output(identity, &opts.global_args.output_format)?;
            } else {
                let output = LongIdentityResponse::new(identity);
                println_output(output, &opts.global_args.output_format)?;
            }
        } else {
            let output = ShortIdentityResponse::new(state.config().identifier().to_string());
            println_output(output, &opts.global_args.output_format)?;
        }
        Ok(())
    }
}

impl Output for LongIdentityResponse<'_> {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        let id: IdentityChangeHistory = serde_bare::from_slice(self.identity.0.as_ref())?;
        write!(w, "{id}")?;
        Ok(w)
    }
}

impl Output for ShortIdentityResponse<'_> {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "{}", self.identity_id)?;
        Ok(w)
    }
}
