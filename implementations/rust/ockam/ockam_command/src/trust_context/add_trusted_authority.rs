use clap::Args;
use indoc::formatdoc;
use itertools::Itertools;
use miette::IntoDiagnostic;

use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_identity::{identities, Identity};
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/add_trusted_authority/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/add_trusted_authority/after_long_help.txt");

/// Add a trusted authority to a trust context
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = false,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct AddTrustedAuthorityCommand {
    pub authority_identity: String,

    /// Optional name of the trust context. Otherwise the default trust context is taken
    #[arg(display_order = 901, long, short)]
    pub name: Option<String>,
}

impl AddTrustedAuthorityCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddTrustedAuthorityCommand),
) -> miette::Result<()> {
    let mut trust_context_state = if let Some(name) = cmd.name {
        opts.state.trust_contexts.get(&name)?
    } else {
        opts.state.trust_contexts.default()?
    };

    let authority_identity = decode_identity(cmd.authority_identity).await?;

    trust_context_state.add_trusted_authority(authority_identity)?;

    let output = formatdoc!(
        r#"Added a new authority identity to the trust context
            Trust Context:
                Name: {}
                ID: {}
                Authority: {}
                Trusted Authorities: {}
            "#,
        trust_context_state.name(),
        trust_context_state.id(),
        trust_context_state.authority_identity().unwrap_or("None"),
        trust_context_state
            .trusted_authorities()
            .into_iter()
            .map(|a| a.to_string())
            .join(", "),
    );

    opts.terminal
        .stdout()
        .plain(output)
        .json(serde_json::to_string_pretty(&trust_context_state.config()).into_diagnostic()?)
        .write_line()?;

    Ok(())
}

async fn decode_identity(identity_as_string: String) -> miette::Result<Identity> {
    let identities_creation = identities().identities_creation();
    identities_creation
        .decode_identity(&hex::decode(identity_as_string).unwrap())
        .await
        .into_diagnostic()
}
