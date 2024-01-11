use clap::Args;
use indoc::formatdoc;
use miette::IntoDiagnostic;

use ockam::identity::Identity;
use ockam_api::cli_state::random_name;
use ockam_core::env::FromString;
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a trust context
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = false,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// The name of the trust context to create
    #[arg(default_value_t = random_name())]
    name: String,

    /// The id of the trust context to create
    #[arg(long)]
    id: Option<String>,

    /// Create a trust context from a credential
    #[arg(long)]
    credential: Option<String>,

    /// Create a trust context from an authority
    #[arg(long)]
    authority_identity: Option<String>,

    /// Create a trust context from an authority
    #[arg(long)]
    authority_route: Option<String>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    let authority = match &cmd.authority_identity {
        None => None,
        Some(identity) => Some(Identity::create(identity).await.into_diagnostic()?),
    };
    let authority_route = cmd
        .authority_route
        .map(|r| MultiAddr::from_string(&r).into_diagnostic())
        .transpose()?;

    let trust_context = opts
        .state
        .create_trust_context(
            Some(cmd.name.clone()),
            cmd.id.clone(),
            cmd.credential,
            authority,
            authority_route,
        )
        .await?;

    let authority = trust_context
        .authority_identity()
        .await
        .into_diagnostic()?
        .map(|i| i.change_history().export_as_string().unwrap())
        .unwrap_or("None".to_string());

    let output = formatdoc!(
        r#"
            Trust Context:
                Name: {}
                ID: {}
                Authority: {}
            "#,
        cmd.name,
        trust_context.trust_context_id(),
        authority
    );

    opts.terminal.stdout().plain(output).write_line()?;
    Ok(())
}
