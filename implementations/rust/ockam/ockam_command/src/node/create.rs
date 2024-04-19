use std::{path::PathBuf, str::FromStr};

use async_trait::async_trait;
use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use tracing::instrument;

use ockam_api::cli_state::random_name;
use ockam_core::{opentelemetry_context_parser, AsyncTryClone, OpenTelemetryContext};
use ockam_node::Context;

use crate::node::create::config::ConfigArgs;
use crate::node::foreground::ForegroundArgs;
use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::util::api::TrustOpts;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::{async_cmd, local_cmd};
use crate::value_parsers::is_url;
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
    /// Name of the node or a configuration to set up the node.
    /// The configuration can be either a path to a local file or a URL.
    #[arg(value_name = "NAME_OR_CONFIG", hide_default_value = true, default_value_t = random_name())]
    pub name: String,

    #[command(flatten)]
    pub config_args: ConfigArgs,

    #[command(flatten)]
    pub foreground_args: ForegroundArgs,

    /// Use this flag to not raise an error if the node is already running.
    /// This can be useful in environments where the PID is constant (e.g., kubernetes).
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    pub skip_is_running_check: bool,

    /// The address to bind the TCP listener to.
    /// Once the node is created, its services can be accessed via this address.
    /// By default, it binds to 127.0.0.1:0 to assign a random free port.
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// A configuration in JSON format to set up the node services.
    /// Node configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(hide = true, long, value_parser = parse_launch_config)]
    pub launch_config: Option<Config>,

    /// The name of an existing Ockam Identity that this node will use.
    /// You can use `ockam identity list` to get a list of existing Identities.
    /// To create a new Identity, use `ockam identity create`.
    /// If you don't specify an Identity name, and you don't have a default Identity, this command
    /// will create a default Identity for you and save it locally in the default Vault
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    pub identity: Option<String>,

    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Serialized opentelemetry context
    #[arg(hide = true, long, value_parser = opentelemetry_context_parser)]
    pub opentelemetry_context: Option<OpenTelemetryContext>,
}

impl Default for CreateCommand {
    fn default() -> Self {
        let node_manager_defaults = NodeManagerDefaults::default();
        Self {
            skip_is_running_check: false,
            name: random_name(),
            config_args: ConfigArgs {
                node_config: None,
                enrollment_ticket: None,
                variables: vec![],
            },
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            launch_config: None,
            identity: None,
            trust_opts: node_manager_defaults.trust_opts,
            opentelemetry_context: None,
            foreground_args: ForegroundArgs {
                foreground: false,
                exit_on_eof: false,
                child_process: false,
            },
        }
    }
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "node create";

    #[instrument(skip_all)]
    fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.parse_args()?;
        if self.has_name_arg() {
            if self.foreground_args.foreground {
                if self.foreground_args.child_process {
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
        self.parse_args()?;
        let ctx = ctx.async_try_clone().await.into_diagnostic()?;
        if self.has_name_arg() {
            if self.foreground_args.foreground {
                self.foreground_mode(&ctx, opts).await?
            } else {
                self.background_mode(&ctx, opts).await?
            }
        } else {
            self.run_config(&ctx, opts).await?
        }
        Ok(())
    }
}

impl CreateCommand {
    pub async fn guard_node_is_not_already_running(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.foreground_args.child_process {
            if let Ok(node) = opts.state.get_node(&self.name).await {
                if node.is_running() {
                    return Err(miette!("Node {} is already running", &self.name));
                }
            }
        }
        Ok(())
    }

    // Return true if the `name` argument is a node name, false if it's a config file path or URL
    // Or if the node configuration was provided inline
    fn has_name_arg(&self) -> bool {
        is_url(&self.name).is_none()
            && std::fs::metadata(&self.name).is_err()
            && self.config_args.node_config.is_none()
    }

    fn parse_args(&self) -> miette::Result<()> {
        // return error if there are duplicated variables
        let mut variables = std::collections::HashMap::new();
        for (key, value) in self.config_args.variables.iter() {
            if variables.contains_key(key) {
                return Err(miette!(
                    "The variable with key '{key}' is duplicated\n\
                Remove the duplicated variable or provide unique keys for each variable"
                ));
            }
            variables.insert(key.clone(), value.clone());
        }
        Ok(())
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
