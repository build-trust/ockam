use clap::Args;

use ockam::Context;
use ockam_api::InfluxDbTokenLease;

use crate::lease::create_project_client;
use crate::output::Output;
use crate::util::api::{CloudOpts, TrustOpts};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

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
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts, trust_opts: TrustOpts) {
        node_rpc(
            opts.rt.clone(),
            run_impl,
            (opts, cloud_opts, self, trust_opts),
        );
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd, trust_opts): (CommandGlobalOpts, CloudOpts, ShowCommand, TrustOpts),
) -> miette::Result<()> {
    let project_node_client = create_project_client(&ctx, &opts, &cloud_opts, &trust_opts).await?;
    let token = project_node_client.get_token(&ctx, cmd.token_id).await?;

    opts.terminal
        .stdout()
        .plain(token.output()?)
        .json(serde_json::json!(&token))
        .write_line()?;

    Ok(())
}
