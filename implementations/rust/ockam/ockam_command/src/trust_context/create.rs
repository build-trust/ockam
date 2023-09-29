use crate::util::local_cmd;
use crate::{docs, util::api::TrustContextOpts, CommandGlobalOpts};
use clap::Args;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};
use ockam_api::cli_state::{random_name, StateDirTrait};

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

    /// Create a trust context from a credential
    #[arg(long)]
    credential: Option<String>,

    #[command(flatten)]
    trust_context_opts: TrustContextOpts,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: CreateCommand) -> miette::Result<()> {
    let config = cmd
        .trust_context_opts
        .to_config(&opts.state)?
        .with_credential_name(cmd.credential.as_ref())
        .use_default_trust_context(false)
        .build();

    if let Some(c) = config {
        opts.state.trust_contexts.create(&cmd.name, c.clone())?;

        let auth = if let Ok(auth) = c.authority() {
            auth.identity_str()
        } else {
            "None"
        };

        let output = formatdoc!(
            r#"
            Trust Context:
                Name: {}
                ID: {}
                Authority: {}
            "#,
            cmd.name,
            c.id(),
            auth
        );

        opts.terminal
            .stdout()
            .plain(output)
            .json(serde_json::to_string_pretty(&c).into_diagnostic()?)
            .write_line()?;
    } else {
        return Err(miette!("Unable to create trust context"));
    }

    Ok(())
}
