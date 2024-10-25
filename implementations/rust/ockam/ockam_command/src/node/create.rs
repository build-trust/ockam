use std::fmt::Write;
use std::{path::PathBuf, str::FromStr};

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use regex::Regex;
use tracing::instrument;

use ockam_api::cli_state::random_name;
use ockam_api::colors::{color_error, color_primary};
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::{opentelemetry_context_parser, OpenTelemetryContext};
use ockam_node::Context;

use crate::node::create::config::ConfigArgs;
use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::shared_args::TrustOpts;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::{async_cmd, local_cmd};
use crate::value_parsers::is_url;
use crate::{docs, Command, CommandGlobalOpts, Result};

pub mod background;
pub mod config;
pub mod foreground;

const DEFAULT_NODE_NAME: &str = "_default_node_name";
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
    #[arg(value_name = "NAME_OR_CONFIGURATION", hide_default_value = true, default_value = DEFAULT_NODE_NAME)]
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

    /// Enable the HTTP server for the node that will listen to in a random free port.
    /// To specify a port, use `--http-server-port` instead.
    #[arg(
        long,
        visible_alias = "enable-http-server",
        value_name = "BOOL",
        default_value_t = false
    )]
    pub http_server: bool,

    /// Enable the HTTP server at the given port.
    #[arg(long, value_name = "PORT", conflicts_with = "http_server")]
    pub http_server_port: Option<u16>,

    /// Enable UDP transport puncture.
    #[arg(
        long,
        visible_alias = "enable-udp",
        value_name = "BOOL",
        default_value_t = false,
        hide = true
    )]
    pub udp: bool,

    /// A configuration in JSON format to set up the node services.
    /// Node configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(hide = true, long, visible_alias = "launch-config", value_parser = parse_launch_config)]
    pub launch_configuration: Option<Config>,

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
            name: DEFAULT_NODE_NAME.to_string(),
            config_args: ConfigArgs {
                configuration: None,
                enrollment_ticket: None,
                variables: vec![],
            },
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            http_server: false,
            http_server_port: None,
            udp: false,
            launch_configuration: None,
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
    fn run(mut self, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.parse_args()?;
        if self.should_run_config() {
            async_cmd(&self.name(), opts.clone(), |ctx| async move {
                self.run_config(&ctx, opts).await
            })
        } else {
            self.set_random_name_if_default();
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
        }
    }

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        self.parse_args()?;
        if self.should_run_config() {
            self.run_config(ctx, opts).await?
        } else {
            self.set_random_name_if_default();
            if self.foreground_args.foreground {
                self.foreground_mode(ctx, opts).await?
            } else {
                self.background_mode(ctx, opts).await?
            }
        }
        Ok(())
    }
}

impl CreateCommand {
    /// Return true if the command should be run in config mode
    fn should_run_config(&self) -> bool {
        let name_arg_is_a_config = !self.has_name_arg();

        let no_config_args = !name_arg_is_a_config
            && self.config_args.configuration.is_none()
            && self.config_args.enrollment_ticket.is_none();
        if no_config_args {
            return false;
        }

        let name_arg_is_default_node_name_or_config =
            self.name.eq(DEFAULT_NODE_NAME) || name_arg_is_a_config;
        name_arg_is_default_node_name_or_config
            || self.config_args.configuration.is_some()
            || self.config_args.enrollment_ticket.is_some()
    }

    /// Return true if the `name` argument is not a config file path or URL
    fn has_name_arg(&self) -> bool {
        let is_file = std::fs::metadata(&self.name)
            .map(|m| m.is_file())
            .unwrap_or(false);
        is_url(&self.name).is_none() && !is_file
    }

    fn parse_args(&self) -> miette::Result<()> {
        // return error if there are duplicated variables
        let mut variables = std::collections::HashMap::new();
        for (key, value) in self.config_args.variables.iter() {
            if variables.contains_key(key) {
                return Err(miette!(
                    "The variable with key {} is duplicated\n\
                Remove the duplicated variable or provide unique keys for each variable",
                    color_primary(key)
                ));
            }
            variables.insert(key.clone(), value.clone());
        }

        // return error if the name arg can't be parsed
        let re = Regex::new(r"[^\w_-]").into_diagnostic()?;
        if self.has_name_arg() && re.is_match(&self.name) {
            return Err(miette!(
                "Invalid value for {}: {}",
                color_primary("NAME_OR_CONFIGURATION"),
                color_error(&self.name),
            ));
        }

        Ok(())
    }

    async fn plain_output(&self, opts: &CommandGlobalOpts, node_name: &str) -> Result<String> {
        let mut buf = String::new();
        writeln!(
            buf,
            "{}",
            fmt_ok!("Created a new Node named {}", color_primary(node_name))
        )?;
        if opts.state.get_node(node_name).await?.is_default() {
            writeln!(
                buf,
                "{}",
                fmt_ok!(
                    "Marked {} as your default Node, on this machine",
                    color_primary(node_name)
                )
            )?;
        }
        writeln!(
            buf,
            "\n{}",
            fmt_log!(
                "To see more details on this Node, run: {}",
                color_primary(format!("ockam node show {}", node_name))
            )
        )?;
        Ok(buf)
    }

    fn set_random_name_if_default(&mut self) {
        if self.name == DEFAULT_NODE_NAME {
            self.name = random_name();
        }
    }

    async fn get_or_create_identity(
        &self,
        opts: &CommandGlobalOpts,
        identity_name: &Option<String>,
    ) -> Result<String> {
        let _notification_handler = NotificationHandler::start(&opts.state, opts.terminal.clone());
        Ok(match identity_name {
            Some(name) => {
                if let Ok(identity) = opts.state.get_named_identity(name).await {
                    identity.name()
                } else {
                    opts.state.create_identity_with_name(name).await?.name()
                }
            }
            None => opts
                .state
                .get_or_create_default_named_identity()
                .await?
                .name(),
        })
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

    #[test]
    fn has_name_arg() {
        // True if it's a node name
        let cmd = CreateCommand::default();
        assert!(cmd.has_name_arg());
        let cmd = CreateCommand {
            name: "node".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.has_name_arg());

        // True if it's a directory-like name
        let cmd = CreateCommand {
            name: "path/to/node".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.has_name_arg());

        // False if it's a file path
        let tmp_directory = tempfile::tempdir().unwrap();
        let tmp_file = tmp_directory.path().join("config.json");
        std::fs::write(&tmp_file, "{}").unwrap();
        let cmd = CreateCommand {
            name: tmp_file.to_str().unwrap().to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.has_name_arg());

        // False if it's a URL
        let cmd = CreateCommand {
            name: "http://localhost:8080".to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.has_name_arg());
    }

    #[test]
    fn should_run_config() {
        let tmp_directory = tempfile::tempdir().unwrap();
        let tmp_file = tmp_directory.path().join("config.json");
        std::fs::write(&tmp_file, "{}").unwrap();
        let config_path = tmp_file.to_str().unwrap().to_string();

        // False with default values
        let cmd = CreateCommand::default();
        assert!(!cmd.should_run_config());

        // True if the name is the default node name and the configuration is set
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                configuration: Some(config_path.clone()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if the name is the default node name and the enrollment ticket is set
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                enrollment_ticket: Some("ticket".to_string()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if the name is not the default node name and the enrollment ticket is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some("ticket".to_string()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if the name is not the default node name and the inline config is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            config_args: ConfigArgs {
                configuration: Some(config_path.clone()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if the name is a file path
        let cmd = CreateCommand {
            name: config_path.clone(),
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if the name is a URL
        let cmd = CreateCommand {
            name: "http://localhost:8080".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // False if the name is a node name and no config is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.should_run_config());
    }
}
