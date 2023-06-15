use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use std::fmt::Write;
use std::str::FromStr;

use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;
use time::format_description::well_known::Iso8601;
use time::PrimitiveDateTime;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::terminal::OckamColor;
use crate::util::output::Output;
use crate::{
    docs,
    util::{
        api::{CloudOpts, TrustContextOpts},
        node_rpc,
        orchestrator_api::OrchestratorApiBuilder,
    },
    CommandGlobalOpts,
};

const HELP_DETAIL: &str = "";

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ListCommand;

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts, trust_opts: TrustContextOpts) {
        initialize_identity_if_default(&opts, &cloud_opts.identity);
        node_rpc(run_impl, (opts, cloud_opts, trust_opts));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, trust_opts): (CommandGlobalOpts, CloudOpts, TrustContextOpts),
) -> miette::Result<()> {
    let identity = get_identity_name(&opts.state, &cloud_opts.identity);
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let mut orchestrator_client = OrchestratorApiBuilder::new(&ctx, &opts, &trust_opts)
            .as_identity(identity)
            .with_new_embbeded_node()
            .await?
            .build(&MultiAddr::from_str("/service/influxdb_token_lease")?)
            .await?;

        let req = Request::get("/");

        let response: Vec<Token> = orchestrator_client.request_with_response(req).await?;
        *is_finished.lock().await = true;
        Ok(response)
    };

    let output_messages = vec![format!("Listing Tokens...\n")];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (tokens, _) = try_join!(send_req, progress_output)?;

    let plain =
        opts.terminal
            .build_list(&tokens, "Tokens", "No active tokens found within service.")?;
    let json = serde_json::to_string_pretty(&tokens).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}

impl Output for Token {
    fn output(&self) -> crate::error::Result<String> {
        let mut output = String::new();
        let status = match self.status.as_str() {
            "active" => self
                .status
                .to_uppercase()
                .color(OckamColor::Success.color()),
            _ => self
                .status
                .to_uppercase()
                .color(OckamColor::Failure.color()),
        };
        let expires_at = {
            PrimitiveDateTime::parse(&self.expires, &Iso8601::DEFAULT)?
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        };
        let id = self
            .id
            .to_string()
            .color(OckamColor::PrimaryResource.color());

        writeln!(output, "Token {id}")?;
        writeln!(output, "Expires {expires_at} {status}")?;
        write!(output, "{}", self.token)?;

        Ok(output)
    }
}
