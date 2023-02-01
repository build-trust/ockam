use std::str::FromStr;

use clap::Args;

use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use termimad::{minimad::TextTemplate, MadSkin};

use crate::{
    help,
    util::{
        api::{CloudOpts, ProjectOpts},
        node_rpc,
        orchestrator_api::OrchestratorApiBuilder,
    },
    CommandGlobalOpts,
};

const HELP_DETAIL: &str = "";

const LIST_VIEW: &str = r#"
## Tokens

${token
> **ID:** ${id}
> **Issued For:** ${issued_for}
> **Created At:** ${created_at}
> **Expires At:** ${expires_at}
> **Token:** ${token}
> **Status:** ${status}


}
"#;

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = help::template(HELP_DETAIL))]
pub struct ListCommand;

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts, cloud_opts: CloudOpts, project_opts: ProjectOpts) {
        node_rpc(run_impl, (options, cloud_opts, project_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, project_opts): (CommandGlobalOpts, CloudOpts, ProjectOpts),
) -> crate::Result<()> {
    let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts, &project_opts)
        .as_identity(cloud_opts.identity.clone())
        .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
        .await?;

    let req = Request::get("/");

    let resp_leases: Vec<Token> = orchestrator_client.request_with_response(req).await?;

    let token_template = TextTemplate::from(LIST_VIEW);
    let mut expander = token_template.expander();

    resp_leases.iter().for_each(
        |Token {
             id,
             issued_for,
             created_at,
             expires,
             token,
             status,
         }| {
            expander
                .sub("token")
                .set("id", id)
                .set("issued_for", issued_for)
                .set("created_at", created_at)
                .set("expires_at", expires)
                .set("token", token)
                .set("status", status);
        },
    );

    let skin = MadSkin::default();

    skin.print_expander(expander);

    Ok(())
}
