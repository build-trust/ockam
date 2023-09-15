use clap::Args;
use miette::IntoDiagnostic;
use termimad::{minimad::TextTemplate, MadSkin};

use ockam::Context;
use ockam_api::InfluxDbTokenLease;

use crate::identity::initialize_identity_if_default;
use crate::lease::authenticate;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

use super::TOKEN_VIEW;

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ShowCommand {
    /// ID of the token to retrieve
    #[arg(short, long, value_name = "TOKEN_ID")]
    pub token_id: String,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts, trust_opts: TrustContextOpts) {
        initialize_identity_if_default(&opts, &cloud_opts.identity);
        node_rpc(run_impl, (opts, cloud_opts, self, trust_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd, trust_opts): (
        CommandGlobalOpts,
        CloudOpts,
        ShowCommand,
        TrustContextOpts,
    ),
) -> miette::Result<()> {
    let project_node = authenticate(&ctx, &opts, &cloud_opts, &trust_opts).await?;
    let token = project_node
        .get_token(&ctx, cmd.token_id)
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    let token_template = TextTemplate::from(TOKEN_VIEW);
    let mut expander = token_template.expander();

    expander
        .set("id", &token.id)
        .set("issued_for", &token.issued_for)
        .set("created_at", &token.created_at)
        .set("expires_at", &token.expires)
        .set("token", &token.token)
        .set("status", &token.status);

    let skin = MadSkin::default();

    skin.print_expander(expander);

    Ok(())
}
