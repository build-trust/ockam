use std::{path::PathBuf, str::FromStr};

use async_trait::async_trait;
use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use tracing::instrument;

use ockam_api::cli_state::{random_name, EnrollmentTicket};
use ockam_core::{opentelemetry_context_parser, AsyncTryClone, OpenTelemetryContext};
use ockam_node::Context;

use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::util::api::TrustOpts;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::{async_cmd, local_cmd};
use crate::value_parsers::{is_url, parse_enrollment_ticket, parse_key_val};
use crate::{docs, Command, CommandGlobalOpts, Result};

pub mod background;
mod config;
pub mod foreground;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the node or path to a config file.
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub name: String,

    /// Run the node in foreground.
    #[arg(display_order = 900, long, short)]
    pub foreground: bool,

    /// Skip the check if such node is already running.
    /// Useful for kubernetes when the pid is the same on each run.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    pub skip_is_running_check: bool,

    /// Watch stdin for EOF
    #[arg(display_order = 900, long, short)]
    pub exit_on_eof: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// `node create` started a child process to run this node in foreground.
    #[arg(long, hide = true)]
    pub child_process: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(long, hide = true, value_parser = parse_launch_config)]
    pub launch_config: Option<Config>,

    /// Name of the Identity that the node will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    pub identity: Option<String>,

    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Serialized opentelemetry context
    #[arg(long, hide = true, value_parser = opentelemetry_context_parser)]
    pub opentelemetry_context: Option<OpenTelemetryContext>,

    /// Path, URL or inlined hex-encoded enrollment ticket
    #[arg(long, value_name = "ENROLLMENT TICKET", value_parser = parse_enrollment_ticket)]
    pub enrollment_ticket: Option<EnrollmentTicket>,

    /// Key-value pairs defining environment variables used by the config file.
    #[arg(long = "variable", value_name = "VARIABLE", value_parser = parse_key_val::<String, String>)]
    pub variables: Vec<(String, String)>,
}

impl Default for CreateCommand {
    fn default() -> Self {
        let node_manager_defaults = NodeManagerDefaults::default();
        Self {
            skip_is_running_check: false,
            name: random_name(),
            exit_on_eof: false,
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            foreground: false,
            child_process: false,
            launch_config: None,
            identity: None,
            trust_opts: node_manager_defaults.trust_opts,
            opentelemetry_context: None,
            enrollment_ticket: None,
            variables: vec![],
        }
    }
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "node create";

    #[instrument(skip_all)]
    fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if self.has_name_arg() {
            if self.foreground {
                if self.child_process {
                    opentelemetry::Context::current()
                        .span()
                        .set_attribute(KeyValue::new("background", "true"));
                }
                local_cmd(embedded_node_that_is_not_stopped(
                    opts.rt.clone(),
                    |ctx| async move { self.foreground_mode(&ctx, opts).await },
                ))
            } else {
                async_cmd(&self.name(), opts.clone(), |ctx| async move {
                    self.background_mode(&ctx, opts).await
                })
            }
        } else {
            return async_cmd(&self.name(), opts.clone(), |ctx| async move {
                self.run_config(&ctx, opts).await
            });
        }
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let ctx = ctx.async_try_clone().await.into_diagnostic()?;
        if self.has_name_arg() {
            if self.foreground {
                self.foreground_mode(&ctx, opts).await?;
            } else {
                self.background_mode(&ctx, opts).await?;
            }
        } else {
            self.run_config(&ctx, opts).await?;
        }
        Ok(())
    }
}

impl CreateCommand {
    pub async fn guard_node_is_not_already_running(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.child_process {
            if let Ok(node) = opts.state.get_node(&self.name).await {
                if node.is_running() {
                    return Err(miette!("Node {} is already running", &self.name));
                }
            }
        }
        Ok(())
    }

    // Return true if the `name` argument is a node name, false if it's a config file path or URL
    fn has_name_arg(&self) -> bool {
        is_url(&self.name).is_none() && std::fs::metadata(&self.name).is_err()
    }
}

pub fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path)
                .into_diagnostic()
                .wrap_err(miette!("Not a valid path"))?;
            Config::read(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }
}
