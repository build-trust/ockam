use crate::node::config::NodeConfig;
use crate::node::create::config::ConfigArgs;
use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::shared_args::TrustOpts;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::print_warning_for_deprecated_flag_no_effect;
use crate::value_parsers::is_url;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_api::cli_state::random_name;
use ockam_api::colors::{color_error, color_primary};
use ockam_api::nodes::models::transport::Port;
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::{opentelemetry_context_parser, OpenTelemetryContext};
use ockam_node::Context;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use regex::Regex;
use std::fmt::Write;
use std::{path::PathBuf, str::FromStr};
use tracing::instrument;

pub mod background;
pub mod config;
pub mod foreground;
pub mod node_callback;

const DEFAULT_NODE_NAME: &str = "_default_node_name";
const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

const DEFAULT_NODE_STATUS_ENDPOINT_PORT: u16 = 23345;

/// Create a new node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the node or a configuration to set up the node.
    /// The configuration can be either a path to a local file or a URL.
    /// TODO: Use Option<String>
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

    /// The address to bind the UDP listener to. UDP listener is not started unless --udp is passed.
    /// Once the node is created, its services can be accessed via this address.
    /// By default, it binds to 127.0.0.1:0 to assign a random free port.
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS_UDP",
        default_value = "127.0.0.1:0"
    )]
    pub udp_listener_address: String,

    /// [DEPRECATED] Enable the HTTP server for the node that will listen to in a random free port.
    /// To specify a port, use `--status-endpoint-port` instead.
    #[arg(
        long,
        visible_alias = "enable-http-server",
        value_name = "BOOL",
        default_value_t = false
    )]
    pub http_server: bool,

    /// Disable the node's status endpoint that serves the healthcheck endpoint.
    #[arg(
        long,
        value_name = "BOOL",
        default_value_t = false,
        conflicts_with = "status_endpoint_port"
    )]
    pub no_status_endpoint: bool,

    /// Specify the port that the status endpoint will listen to.
    #[arg(long, value_name = "PORT")]
    pub status_endpoint_port: Option<u16>,

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

    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    #[arg(help = docs::about("\
    The name of an existing Ockam Identity that this node will use. \
    You can use `ockam identity list` to get a list of existing Identities. \
    To create a new Identity, use `ockam identity create`. \
    If you don't specify an Identity name, and you don't have a default Identity, this command \
    will create a default Identity for you and save it locally in the default Vault
    "))]
    pub identity: Option<String>,

    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Serialized opentelemetry context
    #[arg(hide = true, long, value_parser = opentelemetry_context_parser)]
    pub opentelemetry_context: Option<OpenTelemetryContext>,

    /// Run the node in memory without persisting the state to disk.
    /// It only works with foreground nodes.
    #[arg(
        hide = true,
        long,
        value_name = "BOOL",
        default_value_t = false,
        env = "OCKAM_SQLITE_IN_MEMORY"
    )]
    pub in_memory: bool,

    /// Port that a node should connect to when it's up and running, as a way to signal
    /// the parent process
    #[arg(hide = true, long)]
    pub tcp_callback_port: Option<u16>,
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
                started_from_configuration: false,
            },
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            udp_listener_address: node_manager_defaults.udp_listener_address,
            http_server: false,
            no_status_endpoint: false,
            status_endpoint_port: None,
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
            in_memory: false,
            tcp_callback_port: None,
        }
    }
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "node create";

    #[instrument(skip_all)]
    async fn run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        self.parse_args(&opts).await?;

        if self.should_run_config() {
            self.run_config(ctx, opts).await
        } else if self.foreground_args.foreground {
            if self.foreground_args.child_process {
                opentelemetry::Context::current()
                    .span()
                    .set_attribute(KeyValue::new("background", "true"));
            }

            self.foreground_mode(ctx, opts).await
        } else {
            self.background_mode(opts).await
        }
    }
}

impl CreateCommand {
    /// Return true if the command should be run in config mode
    fn should_run_config(&self) -> bool {
        // Ignore the config args if it's a child process (i.e. only the top level process can run the config)
        if self.foreground_args.child_process {
            return false;
        }

        // Ignore the config args if the node was started from a configuration
        if self.config_args.started_from_configuration {
            return false;
        }

        if !self.name_arg_is_a_config()
            && self.config_args.configuration.is_none()
            && self.config_args.enrollment_ticket.is_none()
        {
            return false;
        }

        true
    }

    /// Return true if the `name` argument is a URL, a file path, or an inline config
    fn name_arg_is_a_config(&self) -> bool {
        let is_url = is_url(&self.name).is_some();
        let is_file = std::fs::metadata(&self.name)
            .map(|m| m.is_file())
            .unwrap_or(false);
        let is_inline_config = serde_yaml::from_str::<NodeConfig>(&self.name).is_ok();
        is_url || is_file || is_inline_config
    }

    fn name_arg_is_a_node_name(&self) -> bool {
        !self.name_arg_is_a_config()
    }

    async fn parse_args(&mut self, opts: &CommandGlobalOpts) -> miette::Result<()> {
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

        // return error if the name arg is not a config and is not a valid node name
        let re = Regex::new(r"[^\w_-]").into_diagnostic()?;
        if self.name_arg_is_a_node_name() && re.is_match(&self.name) {
            return Err(miette!(
                "Invalid value for {}: {}",
                color_primary("NAME_OR_CONFIGURATION"),
                color_error(&self.name),
            ));
        }

        // return error if the name arg is a config and the config arg is also set
        if self.name_arg_is_a_config() && self.config_args.configuration.is_some() {
            return Err(miette!(
                "Cannot set both {} and {}",
                color_primary("NAME_OR_CONFIGURATION"),
                color_primary("--configuration"),
            ));
        }

        // if the name arg is a config, move it to the configuration field and replace
        // the node name with its default value
        if self.name_arg_is_a_config() {
            self.config_args.configuration = Some(self.get_node_config_contents().await?);
            self.name = DEFAULT_NODE_NAME.to_string();
        }

        self.name = {
            let mut name = if let Ok(node_config) = self.parse_node_config().await {
                node_config.node.name().unwrap_or(self.name.clone())
            } else {
                self.name.clone()
            };
            if name == DEFAULT_NODE_NAME {
                name = random_name();
            }
            name
        };

        // FIXME: Avoid this check to avoid parent process needing db to create a background node
        if !self.skip_is_running_check
            && opts
                .state
                .get_node(&self.name)
                .await
                .ok()
                .map(|n| n.is_running())
                .unwrap_or(false)
        {
            return Err(miette!(
                "Node {} is already running",
                color_primary(&self.name)
            ));
        }

        if self.http_server {
            print_warning_for_deprecated_flag_no_effect(opts, "http-server")?;
        }

        Ok(())
    }

    fn status_endpoint_port(&self) -> Option<Port> {
        match (self.no_status_endpoint, self.status_endpoint_port) {
            (true, _) => None,
            (false, Some(port)) => Some(Port::Explicit(port)),
            (false, None) => Some(Port::TryExplicitOrRandom(DEFAULT_NODE_STATUS_ENDPOINT_PORT)),
        }
    }

    async fn plain_output(&self, opts: &CommandGlobalOpts, node_name: &str) -> Result<String> {
        let mut buf = String::new();
        writeln!(
            buf,
            "{}",
            fmt_ok!("Created a new Node named {}", color_primary(node_name))
        )
        .into_diagnostic()?;
        if opts.state.get_node(node_name).await?.is_default() {
            writeln!(
                buf,
                "{}",
                fmt_ok!(
                    "Marked {} as your default Node, on this machine",
                    color_primary(node_name)
                )
            )
            .into_diagnostic()?;
        }

        if self.foreground_args.child_process {
            writeln!(
                buf,
                "\n{}",
                fmt_log!(
                    "To see more details on this Node, run: {}",
                    color_primary(format!("ockam node show {}", node_name))
                )
            )
            .into_diagnostic()?;
        }

        Ok(buf)
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

fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path)
                .into_diagnostic()
                .wrap_err(format!("Invalid path {config_or_path}"))?;
            Config::read(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::resource::utils::parse_cmd_from_args;
    use crate::GlobalArgs;
    use ockam_api::output::OutputFormat;
    use ockam_api::terminal::Terminal;
    use ockam_api::CliState;
    use std::sync::Arc;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }

    #[test]
    fn has_name_arg() {
        // True if it's a node name
        let cmd = CreateCommand::default();
        assert!(cmd.name_arg_is_a_node_name());
        assert!(!cmd.name_arg_is_a_config());
        let cmd = CreateCommand {
            name: "node".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.name_arg_is_a_node_name());
        assert!(!cmd.name_arg_is_a_config());

        // True if it's a directory-like name
        let cmd = CreateCommand {
            name: "path/to/node".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.name_arg_is_a_node_name());
        assert!(!cmd.name_arg_is_a_config());

        // False if it's a file path
        let tmp_directory = tempfile::tempdir().unwrap();
        let tmp_file = tmp_directory.path().join("config.json");
        std::fs::write(&tmp_file, "{}").unwrap();
        let cmd = CreateCommand {
            name: tmp_file.to_str().unwrap().to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.name_arg_is_a_node_name());
        assert!(cmd.name_arg_is_a_config());

        // False if it's a URL
        let cmd = CreateCommand {
            name: "http://localhost:8080".to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.name_arg_is_a_node_name());
        assert!(cmd.name_arg_is_a_config());
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

        // False the name is the default node name, and the configuration is set
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                configuration: Some(config_path.clone()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True the name is the default node name, and the enrollment ticket is set
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                enrollment_ticket: Some("ticket".to_string()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True the name is not the default node name and the enrollment ticket is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some("ticket".to_string()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True the name is not the default node name and the inline config is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            config_args: ConfigArgs {
                configuration: Some(config_path.clone()),
                ..ConfigArgs::default()
            },
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True if foreground and the name is a file path
        let cmd = CreateCommand {
            name: config_path.clone(),
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // True and the name is a URL
        let cmd = CreateCommand {
            name: "http://localhost:8080".to_string(),
            ..CreateCommand::default()
        };
        assert!(cmd.should_run_config());

        // False the name is a node name, and no config is set
        let cmd = CreateCommand {
            name: "node".to_string(),
            ..CreateCommand::default()
        };
        assert!(!cmd.should_run_config());
    }

    #[test]
    fn get_default_node_name_no_previous_state() {
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        rt.block_on(async {
            let opts = CommandGlobalOpts {
                state: CliState::test().await.unwrap(),
                terminal: Terminal::new(
                    false,
                    false,
                    false,
                    true,
                    false,
                    OutputFormat::Plain,
                    "",
                    "",
                ),
                global_args: GlobalArgs::default(),
            };
            let mut cmd = CreateCommand::default();
            cmd.parse_args(&opts).await.unwrap();
            assert_ne!(cmd.name, DEFAULT_NODE_NAME);

            let mut cmd = CreateCommand {
                name: r#"{tcp-outlet: {to: "5500"}}"#.to_string(),
                ..Default::default()
            };
            cmd.parse_args(&opts).await.unwrap();
            assert_ne!(cmd.name, DEFAULT_NODE_NAME);

            let mut cmd = CreateCommand {
                config_args: ConfigArgs {
                    configuration: Some(r#"{tcp-outlet: {to: "5500"}}"#.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            };
            cmd.parse_args(&opts).await.unwrap();
            assert_ne!(cmd.name, DEFAULT_NODE_NAME);

            let mut cmd = CreateCommand {
                name: "n1".to_string(),
                ..Default::default()
            };
            cmd.parse_args(&opts).await.unwrap();
            assert_eq!(cmd.name, "n1");

            let mut cmd = CreateCommand {
                name: "n1".to_string(),
                config_args: ConfigArgs {
                    configuration: Some(r#"{tcp-outlet: {to: "5500"}}"#.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            };
            cmd.parse_args(&opts).await.unwrap();
            assert_eq!(cmd.name, "n1");
        });
    }

    #[ockam::test]
    async fn get_default_node_name_with_previous_state(
        _ctx: &mut Context,
    ) -> ockam_core::Result<()> {
        let opts = CommandGlobalOpts {
            state: CliState::test().await.unwrap(),
            terminal: Terminal::new(
                false,
                false,
                false,
                true,
                false,
                OutputFormat::Plain,
                "",
                "",
            ),
            global_args: GlobalArgs::default(),
        };

        let default_node_name = "n1";
        opts.state.create_node(default_node_name).await.unwrap();

        let mut cmd = CreateCommand::default();
        cmd.parse_args(&opts).await.unwrap();
        assert_ne!(cmd.name, default_node_name);

        let mut cmd = CreateCommand {
            name: "n2".to_string(),
            ..Default::default()
        };
        cmd.parse_args(&opts).await.unwrap();
        assert_eq!(cmd.name, "n2");

        let mut cmd = CreateCommand {
            name: "n2".to_string(),
            config_args: ConfigArgs {
                configuration: Some(r#"{tcp-outlet: {to: "5500"}}"#.to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        cmd.parse_args(&opts).await.unwrap();
        assert_eq!(cmd.name, "n2");

        Ok(())
    }
}
