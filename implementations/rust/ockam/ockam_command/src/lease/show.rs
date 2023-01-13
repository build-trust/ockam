use std::{path::PathBuf, str::FromStr};

use clap::Args;

use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use termimad::{minimad::TextTemplate, MadSkin};

use crate::{
    help,
    util::{node_rpc, orchestrator_api::OrchestratorApiBuilder},
    CommandGlobalOpts,
};

use super::TOKEN_VIEW;

const HELP_DETAIL: &str = "";

/// Show detailed token information within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct ShowCommand {
    /// ID of the token to retrieve
    #[arg(short, long, value_name = "TOKEN_ID")]
    pub token_id: String,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts, identity: Option<String>, project_path: PathBuf) {
        node_rpc(run_impl, (options, identity, project_path, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, identity, project_path, cmd): (CommandGlobalOpts, Option<String>, PathBuf, ShowCommand),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts)
        .as_identity(identity)
        .with_new_embbeded_node()
        .await?
        .with_project_from_file(&project_path)
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req = Request::get(format!("/{}", cmd.token_id));

    let resp_token: Token = orchestrator_client.request_with_response(req).await?;

    let token_template = TextTemplate::from(TOKEN_VIEW);
    let mut expander = token_template.expander();

    expander
        .set("id", &resp_token.id)
        .set("issued_for", &resp_token.issued_for)
        .set("created_at", &resp_token.created_at)
        .set("expires_at", &resp_token.expires)
        .set("token", &resp_token.token)
        .set("status", &resp_token.status);

    let skin = MadSkin::default();

    skin.print_expander(expander);

    Ok(())
}
