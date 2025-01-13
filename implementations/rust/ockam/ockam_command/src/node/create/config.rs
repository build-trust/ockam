use crate::node::show::is_node_up;
use crate::node::CreateCommand;
use crate::run::parser::config::ConfigParser;
use crate::run::parser::resource::*;
use crate::run::parser::Version;
use crate::value_parsers::{parse_config_or_path_or_url, parse_key_val};
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::{miette, IntoDiagnostic};
use ockam_api::cli_state::journeys::APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::OpenTelemetryContext;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace, Span};

pub const ENROLLMENT_TICKET: &str = "ENROLLMENT_TICKET";

#[derive(Clone, Debug, Args, Default)]
pub struct ConfigArgs {
    /// Inline node configuration
    #[arg(long, visible_alias = "node-config", value_name = "YAML")]
    pub configuration: Option<String>,

    #[arg(long, env = "ENROLLMENT_TICKET", value_name = "ENROLLMENT TICKET")]
    #[arg(help = docs::about("\
    A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node. \
    When passed, the identity will be given a project membership credential. \
    Check the `project ticket` command for more information about enrollment tickets.
    "))]
    pub enrollment_ticket: Option<String>,

    /// Key-value pairs defining environment variables used in the Node configuration.
    /// The variables passed here will have precedence over global environment variables.
    /// This argument can be used multiple times, each time adding a new key-value pair.
    /// Example: `--variable KEY1=VALUE1 --variable KEY2=VALUE2`
    #[arg(long = "variable", value_name = "VARIABLE", value_parser = parse_key_val::<String, String>)]
    pub variables: Vec<(String, String)>,

    /// A flag used internally to indicate that the node was started from a configuration file.
    #[arg(hide = true, long)]
    pub started_from_configuration: bool,
}

impl CreateCommand {
    /// Run the creation of a node using a node configuration
    #[instrument(skip_all)]
    pub async fn run_config(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        debug!("running node create with a node config");
        let mut node_config = self.parse_node_config().await?;
        node_config.merge(&self).await?;
        trace!(?node_config, "merged node config with command args");
        let node_name = node_config.node.name().ok_or(miette!(
            "Node name should be set to the command's default value"
        ))?;
        let identity_name = self
            .get_or_create_identity(&opts, &node_config.node.identity())
            .await?;
        let res = if self.foreground_args.foreground {
            node_config
                .run_foreground(ctx, &opts, &node_name, &identity_name)
                .await
        } else {
            node_config
                .run_background(ctx, &opts, &node_name, &identity_name)
                .await
        };
        if res.is_err() {
            let _ = opts.state.delete_node(&node_name).await;
        }
        res
    }

    pub(super) async fn get_node_config_contents(&self) -> miette::Result<String> {
        match self.config_args.configuration.clone() {
            Some(contents) => Ok(contents),
            None => match parse_config_or_path_or_url::<NodeConfig>(&self.name).await {
                Ok(contents) => Ok(contents),
                Err(err) => {
                    // If just the enrollment ticket is passed, create a minimal configuration
                    if let Some(ticket) = &self.config_args.enrollment_ticket {
                        Ok(format!("ticket: {}", ticket))
                    } else {
                        Err(err)
                    }
                }
            },
        }
    }

    /// Try to read the `name` argument as either:
    ///  - a URL to a configuration file
    ///  - a local path to a configuration file
    ///  - an inline configuration
    /// or read the `configuration` argument
    #[instrument(skip_all, fields(app.event.command.configuration_file))]
    pub(super) async fn parse_node_config(&self) -> miette::Result<NodeConfig> {
        let contents = self.get_node_config_contents().await?;
        // Set environment variables from the cli command args
        // This needs to be done before parsing the configuration
        for (key, value) in &self.config_args.variables {
            if value.is_empty() {
                return Err(miette!("Empty value for variable '{key}'"));
            }
            std::env::set_var(key, value);
        }
        // Parse the configuration
        let node_config = NodeConfig::parse(contents.clone())?;
        // Record the configuration contents if the node configuration was successfully parsed
        Span::current().record(
            APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE.as_str(),
            contents.to_string(),
        );
        Ok(node_config)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    #[serde(flatten)]
    pub version: Version,
    #[serde(flatten)]
    pub project_enroll: ProjectEnroll,
    #[serde(flatten)]
    pub node: Node,
    #[serde(flatten)]
    pub policies: Policies,
    #[serde(flatten)]
    pub relays: Relays,
    #[serde(flatten)]
    pub tcp_outlets: TcpOutlets,
    #[serde(flatten)]
    pub tcp_inlets: TcpInlets,
    #[serde(flatten)]
    pub influxdb_inlets: InfluxDBInlets,
    #[serde(flatten)]
    pub influxdb_outlets: InfluxDBOutlets,
    #[serde(flatten)]
    pub kafka_inlet: KafkaInlet,
    #[serde(flatten)]
    pub kafka_outlet: KafkaOutlet,
}

impl NodeConfig {
    fn parse(mut contents: String) -> miette::Result<Self> {
        ConfigParser::parse(&mut contents)
    }

    /// Merge the arguments of the node defined in the config with the arguments from the
    /// "create" command, giving precedence to the command args.
    async fn merge(&mut self, cmd: &CreateCommand) -> miette::Result<()> {
        // Set environment variables from the cli command again
        // to override the duplicate entries from the config file.
        for (key, value) in &cmd.config_args.variables {
            if value.is_empty() {
                return Err(miette!("Empty value for variable '{key}'"));
            }
            std::env::set_var(key, value);
        }

        // Set default values to the config, if not present
        if self.node.name.is_none() {
            self.node.name = Some(cmd.name.clone().into());
        }
        if self.node.opentelemetry_context.is_none() {
            self.node.opentelemetry_context = Some(
                serde_json::to_string(&OpenTelemetryContext::current())
                    .into_diagnostic()?
                    .into(),
            );
        }

        // Override config values with passed command args
        let default_cmd_args = CreateCommand::default();
        if cmd.name.ne(&default_cmd_args.name) && cmd.name_arg_is_a_node_name() {
            self.node.name = Some(cmd.name.clone().into());
        }
        if let Some(ticket) = &cmd.config_args.enrollment_ticket {
            self.project_enroll.ticket = Some(ticket.clone());
        }
        if cmd.skip_is_running_check != default_cmd_args.skip_is_running_check {
            self.node.skip_is_running_check = Some(cmd.skip_is_running_check.into());
        }
        if cmd.foreground_args.foreground != default_cmd_args.foreground_args.foreground {
            self.node.foreground = Some(cmd.foreground_args.foreground.into());
        }
        if cmd.foreground_args.child_process != default_cmd_args.foreground_args.child_process {
            self.node.child_process = Some(cmd.foreground_args.child_process.into());
        }
        if cmd.foreground_args.exit_on_eof != default_cmd_args.foreground_args.exit_on_eof {
            self.node.exit_on_eof = Some(cmd.foreground_args.exit_on_eof.into());
        }
        if cmd.tcp_listener_address != default_cmd_args.tcp_listener_address {
            self.node.tcp_listener_address = Some(cmd.tcp_listener_address.clone().into());
        }
        if cmd.udp_listener_address != default_cmd_args.udp_listener_address {
            self.node.udp_listener_address = Some(cmd.udp_listener_address.clone().into());
        }
        if cmd.no_status_endpoint != default_cmd_args.no_status_endpoint {
            self.node.no_status_endpoint = Some(cmd.no_status_endpoint.into());
        }
        if let Some(port) = cmd.status_endpoint_port {
            self.node.status_endpoint_port = Some((port as isize).into());
        }
        if let Some(identity) = &cmd.identity {
            self.node.identity = Some(identity.clone().into());
        }
        if let Some(project) = &cmd.trust_opts.project_name {
            self.node.project = Some(project.clone().into());
        }
        if let Some(launch_config) = &cmd.launch_configuration {
            self.node.launch_config = Some(
                serde_json::to_string(launch_config)
                    .into_diagnostic()?
                    .into(),
            );
        }
        if let Some(context) = &cmd.opentelemetry_context {
            self.node.opentelemetry_context =
                Some(serde_json::to_string(&context).into_diagnostic()?.into());
        }
        if cmd.udp != default_cmd_args.udp {
            self.node.udp = Some(cmd.udp.into());
        }

        Ok(())
    }

    async fn run_foreground(
        self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        node_name: &String,
        identity_name: &String,
    ) -> miette::Result<()> {
        debug!("Running node config in foreground mode");
        // First, run the `project enroll` commands to prepare the identity and project data
        if let Some(command) = self
            .project_enroll
            .into_parsed_commands(Some(identity_name))?
            .into_iter()
            .next()
        {
            command.run(ctx, opts).await?;
            // Newline before the `node create` command
            opts.terminal.write_line("")?;
        }

        // Next, run the 'node create' command in a separate tokio task,
        // where the foreground node will run until stopped
        let node_handle = {
            let node_command = self
                .node
                .into_parsed_commands()?
                .into_iter()
                .next()
                .ok_or(miette!("A node command should be defined"))?;
            let opts = opts.clone();
            tokio::task::spawn_blocking(move || crate::Command::run(node_command, opts))
        };

        // Wait for the node to be up
        let is_up = {
            let mut node = BackgroundNodeClient::create_to_node(ctx, &opts.state, node_name)?;
            is_node_up(ctx, &mut node, true).await?
        };
        if !is_up {
            return Err(miette!("Node failed to start"));
        }

        // Run the other sections
        let node_name = Some(node_name);
        let other_sections: Vec<ParsedCommands> = vec![
            self.policies.into_parsed_commands()?.into(),
            self.relays.into_parsed_commands(node_name)?.into(),
            self.tcp_outlets.into_parsed_commands(node_name)?.into(),
            self.tcp_inlets.into_parsed_commands(node_name)?.into(),
            self.influxdb_outlets
                .into_parsed_commands(node_name)?
                .into(),
            self.influxdb_inlets.into_parsed_commands(node_name)?.into(),
            self.kafka_outlet.into_parsed_commands(node_name)?.into(),
            self.kafka_inlet.into_parsed_commands(node_name)?.into(),
        ];
        opts.terminal.write_line("")?;
        Self::run_commands_sections(ctx, opts, other_sections).await?;
        opts.terminal.write_line("")?;

        // Block on the node until it exits
        let _ = node_handle.await.into_diagnostic()?;
        Ok(())
    }

    async fn run_background(
        self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        node_name: &String,
        identity_name: &String,
    ) -> miette::Result<()> {
        let sections = self.parse_commands(node_name, identity_name)?;
        Self::run_commands_sections(ctx, opts, sections).await?;
        Ok(())
    }

    /// Build commands and return validation errors if any
    fn parse_commands(
        self,
        node_name: &String,
        identity_name: &String,
    ) -> miette::Result<Vec<ParsedCommands>> {
        let node_name = Some(node_name);
        let identity_name = Some(identity_name);
        Ok(vec![
            self.project_enroll
                .into_parsed_commands(identity_name)?
                .into(),
            self.node.into_parsed_commands()?.into(),
            self.policies.into_parsed_commands()?.into(),
            self.relays.into_parsed_commands(node_name)?.into(),
            self.tcp_outlets.into_parsed_commands(node_name)?.into(),
            self.tcp_inlets.into_parsed_commands(node_name)?.into(),
            self.influxdb_outlets
                .into_parsed_commands(node_name)?
                .into(),
            self.influxdb_inlets.into_parsed_commands(node_name)?.into(),
            self.kafka_outlet.into_parsed_commands(node_name)?.into(),
            self.kafka_inlet.into_parsed_commands(node_name)?.into(),
        ])
    }

    async fn run_commands_sections(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        sections: Vec<ParsedCommands>,
    ) -> miette::Result<()> {
        let sections: Vec<ParsedCommands> = sections
            .into_iter()
            .filter(|s| !s.commands.is_empty())
            .collect();
        let len = sections.len();
        for (idx, section) in sections.into_iter().enumerate() {
            if section.commands.is_empty() {
                continue;
            }
            section.run(ctx, opts).await?;
            if idx < len - 1 {
                opts.terminal.write_line("")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::cli_state::ExportedEnrollmentTicket;

    #[tokio::test]
    async fn get_node_config_from_path() {
        let config = "name: n1";
        let dummy_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(dummy_file.path(), config).unwrap();
        let cmd = CreateCommand {
            name: dummy_file.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        let res = cmd.parse_node_config().await.unwrap();
        assert_eq!(res.node.name, Some("n1".into()));
    }

    #[tokio::test]
    async fn get_node_config_from_url() {
        let mut server = mockito::Server::new_async().await;
        let config_url = format!("{}/config.yaml", server.url());
        server
            .mock("GET", "/config.yaml")
            .with_status(201)
            .with_header("content-type", "text/plain")
            .with_body("name: n1")
            .create_async()
            .await;
        let cmd = CreateCommand {
            name: config_url,
            ..Default::default()
        };
        let res = cmd.parse_node_config().await.unwrap();
        assert_eq!(res.node.name, Some("n1".into()));
    }

    #[tokio::test]
    async fn get_node_config_from_inline() {
        let config = "name: n1";
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                configuration: Some(config.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let res = cmd.parse_node_config().await.unwrap();
        assert_eq!(res.node.name, Some("n1".into()));
    }

    #[tokio::test]
    async fn get_node_config_from_enrollment_ticket() {
        let ticket = ExportedEnrollmentTicket::new_test();
        let ticket_encoded = ticket.to_string();
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                enrollment_ticket: Some(ticket_encoded.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let res = cmd.parse_node_config().await.unwrap();
        assert_eq!(res.project_enroll.ticket, Some(ticket_encoded));
    }

    #[test]
    fn parse_demo_config_files() {
        let demo_files_dir = std::env::current_dir()
            .unwrap()
            .join("src")
            .join("node")
            .join("create")
            .join("demo_config_files");
        let files = std::fs::read_dir(demo_files_dir).unwrap();
        for file in files {
            let file = file.unwrap();
            let path = file.path();
            let contents = std::fs::read_to_string(&path).unwrap();
            let res = NodeConfig::parse(contents);
            res.unwrap();
        }
    }

    #[tokio::test]
    async fn node_name_is_handled_correctly() {
        // The command doesn't define a node name, the config file does
        let tmp_directory = tempfile::tempdir().unwrap();
        let tmp_file = tmp_directory.path().join("config.json");
        std::fs::write(&tmp_file, "{name: n1}").unwrap();
        let cmd = CreateCommand {
            name: tmp_file.to_str().unwrap().to_string(),
            ..Default::default()
        };
        let mut config = cmd.parse_node_config().await.unwrap();
        config.merge(&cmd).await.unwrap();
        assert_eq!(config.node.name, Some("n1".into()));

        // Same with inline config
        let cmd = CreateCommand {
            config_args: ConfigArgs {
                configuration: Some("{name: n1}".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut config = cmd.parse_node_config().await.unwrap();
        config.merge(&cmd).await.unwrap();
        assert_eq!(config.node.name, Some("n1".into()));

        // If the command defines a node name, it should override the inline config
        let cmd = CreateCommand {
            name: "n2".into(),
            config_args: ConfigArgs {
                configuration: Some("{name: n1}".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut config = cmd.parse_node_config().await.unwrap();
        config.merge(&cmd).await.unwrap();
        assert_eq!(config.node.name, Some("n2".into()));
    }

    #[tokio::test]
    async fn merge_config_with_cli() {
        let cli_enrollment_ticket = ExportedEnrollmentTicket::new_test();
        let cli_enrollment_ticket_encoded = cli_enrollment_ticket.to_string();

        let cli_args = CreateCommand {
            tcp_listener_address: "127.0.0.1:1234".to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(cli_enrollment_ticket_encoded.clone()),
                ..Default::default()
            },
            ..Default::default()
        };

        // No node config, cli args should be used
        let contents = String::new();
        let mut config = NodeConfig::parse(contents).unwrap();
        config.merge(&cli_args).await.unwrap();
        let node = config.node.into_parsed_commands().unwrap().pop().unwrap();
        assert_eq!(node.tcp_listener_address, "127.0.0.1:1234");
        assert_eq!(
            config.project_enroll.ticket,
            Some(cli_enrollment_ticket_encoded.clone())
        );

        // Config used, cli args should override the overlapping args
        let config_enrollment_ticket = ExportedEnrollmentTicket::new_test();
        let config_enrollment_ticket_encoded = config_enrollment_ticket.to_string();
        std::env::set_var(ENROLLMENT_TICKET, config_enrollment_ticket_encoded);

        let contents = r#"
            ticket: $ENROLLMENT_TICKET
            name: n1
            tcp-listener-address: 127.0.0.1:5555
        "#
        .to_string();
        let mut config = NodeConfig::parse(contents).unwrap();
        config.merge(&cli_args).await.unwrap();
        let node = config.node.into_parsed_commands().unwrap().pop().unwrap();
        assert_eq!(node.name, "n1");
        assert_eq!(node.tcp_listener_address, cli_args.tcp_listener_address);
        assert_eq!(
            config.project_enroll.ticket,
            Some(cli_enrollment_ticket_encoded.clone())
        );
    }
}
