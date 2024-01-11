use clap::{arg, Args};
use colorful::Colorful;
use indoc::formatdoc;
use ockam::Context;

use crate::output::CredentialAndPurposeKeyDisplay;
use crate::{util::node_rpc, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[arg()]
    pub credential_name: String,

    /// Name of the Vault from which to retrieve the credential
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let named_credential = opts
        .state
        .get_credential_by_name(&cmd.credential_name)
        .await?;

    let is_verified = "✔︎".light_green();
    let credential = named_credential.credential_and_purpose_key();
    let plain = formatdoc!(
        r#"
        Credential: {} {is_verified}
        {}
        "#,
        &cmd.credential_name,
        CredentialAndPurposeKeyDisplay(credential)
    );

    opts.terminal.stdout().plain(plain).write_line()?;

    Ok(())
}
