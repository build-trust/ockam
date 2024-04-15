use crate::node::CreateCommand;
use crate::run::parser::building_blocks::ArgValue;
use crate::run::parser::config::ConfigParser;
use crate::run::parser::resource::*;
use crate::run::parser::Version;
use crate::value_parsers::async_parse_path_or_url;
use crate::CommandGlobalOpts;
use ockam_api::cli_state::journeys::APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::{instrument, Span};

impl CreateCommand {
    /// Run the creation of a node using a node configuration
    #[instrument(skip_all)]
    pub async fn run_config(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let mut node_config = self.get_node_config().await?;
        node_config.merge(&self)?;
        node_config.run(ctx, &opts).await?;

        if self.foreground {
            self.wait_for_exit_signal(ctx, opts).await?;
        }

        Ok(())
    }

    /// Try to read the self.name field as either:
    ///  - a URL to a configuration file
    ///  - a local path to a configuration file
    ///  - an inline configuration
    #[instrument(skip_all, fields(app.event.command.configuration_file))]
    pub async fn get_node_config(&self) -> miette::Result<NodeConfig> {
        let contents = match self.node_config.clone() {
            Some(contents) => contents,
            None => async_parse_path_or_url(&self.name).await?,
        };
        // Set environment variables from the cli command args
        // This needs to be done before parsing the configuration
        for (key, value) in &self.variables {
            std::env::set_var(key, value);
        }
        // Parse the configuration
        let node_config = NodeConfig::new(&contents)?;
        // Record the configuration contents if the node configuration was successfully parsed
        Span::current().record(
            APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE.as_str(),
            &contents.to_string(),
        );
        Ok(node_config)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
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
    pub tcp_outlets: TcpOutlets,
    #[serde(flatten)]
    pub tcp_inlets: TcpInlets,
    #[serde(flatten)]
    pub kafka_inlet: KafkaInlet,
    #[serde(flatten)]
    pub kafka_outlet: KafkaOutlet,
    #[serde(flatten)]
    pub relays: Relays,
}

impl ConfigParser<'_> for NodeConfig {}

impl NodeConfig {
    fn new(contents: &str) -> miette::Result<Self> {
        Self::parse(&Self::resolve(contents)?)
    }

    /// Merge the arguments of the node defined in the config with the arguments from the
    /// "create" command, giving precedence to the config values.
    fn merge(&mut self, cli_args: &CreateCommand) -> miette::Result<()> {
        // Set environment variables from the cli command again
        // to override the duplicate entries from the config file.
        for (key, value) in &cli_args.variables {
            std::env::set_var(key, value);
        }

        // Set the enrollment ticket from the cli command
        // overriding the one from the config file.
        if let Some(ticket) = &cli_args.enrollment_ticket {
            self.project_enroll.ticket = Some(ticket.hex_encoded()?);
        }

        // Merge the node arguments from the config with the cli command args.
        if self.node.skip_is_running_check.is_none() {
            self.node.skip_is_running_check = Some(ArgValue::Bool(cli_args.skip_is_running_check));
        }
        if self.node.exit_on_eof.is_none() {
            self.node.exit_on_eof = Some(ArgValue::Bool(cli_args.exit_on_eof));
        }
        if self.node.tcp_listener_address.is_none() {
            self.node.tcp_listener_address =
                Some(ArgValue::String(cli_args.tcp_listener_address.clone()));
        }
        if self.node.identity.is_none() {
            self.node.identity = cli_args.identity.clone().map(ArgValue::String);
        }
        if self.node.project.is_none() {
            self.node.project = cli_args
                .trust_opts
                .project_name
                .clone()
                .map(ArgValue::String);
        }
        Ok(())
    }

    pub async fn run(self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        // Parse then run commands
        for cmd in self.parse_commands()? {
            cmd.run(ctx, opts).await?
        }
        Ok(())
    }

    /// Build commands and return validation errors if any
    pub fn parse_commands(self) -> miette::Result<Vec<ParsedCommands>> {
        let node_name = self.node.name();
        Ok(vec![
            self.project_enroll.parse_commands()?.into(),
            self.node.parse_commands()?.into(),
            self.relays.parse_commands(&node_name)?.into(),
            self.policies.parse_commands()?.into(),
            self.tcp_outlets.parse_commands(&node_name)?.into(),
            self.tcp_inlets.parse_commands(&node_name)?.into(),
            self.kafka_inlet.parse_commands(&node_name)?.into(),
            self.kafka_outlet.parse_commands(&node_name)?.into(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::cli_state::EnrollmentTicket;

    use super::*;

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
            let res = NodeConfig::parse(&contents);
            res.unwrap();
        }
    }

    #[test]
    fn merge_config_with_cli() {
        let enrollment_ticket = EnrollmentTicket::new(OneTimeCode::new(), None);
        let enrollment_ticket_hex = enrollment_ticket.hex_encoded().unwrap();
        std::env::set_var("ENROLLMENT_TICKET", &enrollment_ticket_hex);

        let cli_args = CreateCommand {
            tcp_listener_address: "127.0.0.1:1234".to_string(),
            enrollment_ticket: Some(enrollment_ticket.clone()),
            ..Default::default()
        };

        // No node config, cli args should be used
        let mut config = NodeConfig::parse("").unwrap();
        config.merge(&cli_args).unwrap();
        let node = config.node.parse_commands().unwrap().pop().unwrap();
        assert_eq!(node.tcp_listener_address, "127.0.0.1:1234");
        assert_eq!(
            config.project_enroll.ticket,
            Some(enrollment_ticket_hex.clone())
        );

        // Config used, cli args should be ignored
        let mut config = NodeConfig::parse(
            r#"
            ticket: $ENROLLMENT_TICKET
            name: n1
            tcp-listener-address: 127.0.0.1:5555
        "#,
        )
        .unwrap();
        config.merge(&cli_args).unwrap();
        let node = config.node.parse_commands().unwrap().pop().unwrap();
        assert_eq!(node.name, "n1");
        assert_eq!(node.tcp_listener_address, "127.0.0.1:5555".to_string());
        assert_eq!(
            config.project_enroll.ticket,
            Some(enrollment_ticket_hex.clone())
        );
    }
}
