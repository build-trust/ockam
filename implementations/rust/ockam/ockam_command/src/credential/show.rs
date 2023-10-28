use clap::{arg, Args};
use colorful::Colorful;
use indoc::formatdoc;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

use crate::credential::identities;
use crate::output::CredentialAndPurposeKeyDisplay;
use crate::{
    credential::validate_encoded_cred, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
};

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
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));

    let cred_name = &cmd.credential_name;
    let cred = opts.state.credentials.get(cred_name)?;
    let cred_config = cred.config();

    let identities = identities(&vault_name, &opts).await?;
    identities
        .identities_creation()
        .import(
            Some(&cred_config.issuer_identifier),
            &cred_config.encoded_issuer_change_history,
        )
        .await
        .into_diagnostic()?;

    let is_verified = match validate_encoded_cred(
        &cred_config.encoded_credential,
        identities,
        &cred_config.issuer_identifier,
    )
    .await
    {
        Ok(_) => "✔︎".light_green(),
        Err(_) => "✕".light_red(),
    };

    let cred = cred_config.credential()?;
    let plain = formatdoc!(
        r#"
        Credential: {cred_name} {is_verified}
        {}
        "#,
        CredentialAndPurposeKeyDisplay(cred)
    );

    opts.terminal.stdout().plain(plain).write_line()?;

    Ok(())
}
