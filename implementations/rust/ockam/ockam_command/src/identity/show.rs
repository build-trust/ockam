use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::output::{EncodeFormat, IdentifierDisplay, IdentityDisplay};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::IntoDiagnostic;
use ockam::identity::{Identity, Vault};
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of an identity
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[arg()]
    name: Option<String>,

    /// Show the full identity history, and not just the identifier or the name
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
        let identifier = state.config().identifier();
        if cmd.full {
            let change_history = opts
                .state
                .identities
                .identities_repository()
                .await?
                .get_identity(&identifier)
                .await
                .into_diagnostic()?;

            if Some(EncodeFormat::Hex) == cmd.encoding {
                opts.println(&hex::encode(change_history.export().into_diagnostic()?))?;
            } else {
                let identity = Identity::import_from_change_history(
                    Some(&identifier),
                    change_history,
                    Vault::create_verifying_vault(),
                )
                .await
                .into_diagnostic()?;

                let identity_display = IdentityDisplay(identity);
                opts.println(&identity_display)?;
            }
        } else {
            let identifier_display = IdentifierDisplay(identifier);
            opts.println(&identifier_display)?;
        }
        Ok(())
    }
}
