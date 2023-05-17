use std::str::FromStr;

use clap::Args;

use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use termimad::{minimad::TextTemplate, MadSkin};

use crate::identity::get_identity_name;
use crate::{
    docs,
    util::{
        api::{CloudOpts, TrustContextOpts},
        node_rpc,
        orchestrator_api::OrchestratorApiBuilder,
    },
    CommandGlobalOpts,
};

use super::TOKEN_VIEW;

const HELP_DETAIL: &str = "";

/// Create a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct CreateCommand {}

impl CreateCommand {
    pub fn run(
        self,
        options: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustContextOpts,
    ) {
        node_rpc(run_impl, (options, cloud_opts, trust_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, trust_opts): (CommandGlobalOpts, CloudOpts, TrustContextOpts),
) -> crate::Result<()> {
    let identity = get_identity_name(&opts.state, cloud_opts.identity.clone())?;
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts, &trust_opts)
        .as_identity(identity)
        .with_new_embbeded_node()
        .await?
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req = Request::post("/");

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
