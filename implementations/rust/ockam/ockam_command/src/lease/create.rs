use clap::Args;

use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::InfluxDbTokenLease;
use time::format_description::well_known::Iso8601;
use time::PrimitiveDateTime;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::lease::create_project_client;
use crate::terminal::OckamColor;
use crate::util::api::{CloudOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};
use crate::{fmt_log, fmt_ok};

const HELP_DETAIL: &str = "";

/// Create a token within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct CreateCommand {}

impl CreateCommand {
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, cloud_opts, trust_opts).await
        })
    }

    pub fn name(&self) -> String {
        "create token".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        cloud_opts: CloudOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        opts.terminal
            .write_line(&fmt_log!("Creating influxdb token...\n"))?;

        let project_node = create_project_client(ctx, &opts, &cloud_opts, &trust_opts).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let token = project_node.create_token(ctx).await?;
            *is_finished.lock().await = true;
            Ok(token)
        };

        let output_messages = vec!["Creating influxdb token...".to_string()];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (resp_token, _) = try_join!(send_req, progress_output)?;

        opts.terminal
            .stdout()
            .machine(resp_token.token.to_string())
            .json(serde_json::to_string_pretty(&resp_token).into_diagnostic()?)
            .plain(
                fmt_ok!("Created influxdb token\n")
                    + &fmt_log!(
                        "{}\n",
                        &resp_token
                            .token
                            .to_string()
                            .color(OckamColor::PrimaryResource.color())
                    )
                    + &fmt_log!(
                        "Id {}\n",
                        &resp_token
                            .id
                            .to_string()
                            .color(OckamColor::PrimaryResource.color())
                    )
                    + &fmt_log!(
                        "Expires at {}\n",
                        PrimitiveDateTime::parse(&resp_token.expires, &Iso8601::DEFAULT)
                            .into_diagnostic()?
                            .to_string()
                            .color(OckamColor::PrimaryResource.color())
                    ),
            )
            .write_line()?;

        Ok(())
    }
}
