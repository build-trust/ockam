use clap::Args;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cloud::lease_manager::models::influxdb::Token;
use ockam_api::InfluxDbTokenLeaseManagerTrait;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::lease::create_project_client;
use crate::shared_args::{IdentityOpts, TrustOpts};
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

/// List tokens within the lease token manager
#[derive(Clone, Debug, Args)]
#[command(help_template = docs::after_help(HELP_DETAIL))]
pub struct ListCommand;

impl ListCommand {
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        identity_opts: IdentityOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts, identity_opts, trust_opts).await
        })
    }

    pub fn name(&self) -> String {
        "lease list".into()
    }

    async fn async_run(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        identity_opts: IdentityOpts,
        trust_opts: TrustOpts,
    ) -> miette::Result<()> {
        let is_finished: Mutex<bool> = Mutex::new(false);
        let project_node = create_project_client(ctx, &opts, &identity_opts, &trust_opts).await?;

        let send_req = async {
            let tokens: Vec<Token> = project_node.list_tokens(ctx).await?;
            *is_finished.lock().await = true;
            Ok(tokens)
        };

        let output_messages = vec![format!("Listing Tokens...\n")];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (tokens, _) = try_join!(send_req, progress_output)?;

        let plain = opts
            .terminal
            .build_list(&tokens, "No active tokens found within service.")?;
        let json = serde_json::to_string(&tokens).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json(json)
            .write_line()?;
        Ok(())
    }
}
