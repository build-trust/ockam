use crate::node::show::is_node_up;
use crate::node::CreateCommand;
use crate::run::parser::building_blocks::ArgValue;
use crate::run::parser::config::ConfigParser;

use crate::run::parser::resource::*;
use crate::run::parser::Version;
use crate::value_parsers::{async_parse_path_or_url, parse_enrollment_ticket, parse_key_val};
use crate::CommandGlobalOpts;
use clap::Args;

use miette::{miette, IntoDiagnostic};
use ockam_api::cli_state::journeys::APPLICATION_EVENT_COMMAND_CONFIGURATION_FILE;
use ockam_api::cli_state::{random_name, EnrollmentTicket};

use ockam_api::nodes::BackgroundNodeClient;

use ockam_core::AsyncTryClone;
use ockam_node::Context;
use serde::{Deserialize, Serialize};

use tracing::{debug, instrument, Span};

#[derive(Clone, Debug, Args, Default)]
pub struct ConfigArgs {
    /// Inline node configuration
    #[arg(long, value_name = "YAML")]
    pub node_config: Option<String>,

    /// A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node.
    /// When passed, the identity will be given a project membership credential.
    /// Check the `project ticket` command for more information about enrollment tickets.
    #[arg(long, value_name = "ENROLLMENT TICKET", value_parser = parse_enrollment_ticket)]
    pub enrollment_ticket: Option<EnrollmentTicket>,

    /// Key-value pairs defining environment variables used in the Node configuration.
    /// The variables passed here will have precedence over global environment variables.
    /// This argument can be used multiple times, each time adding a new key-value pair.
    /// Example: `--variable KEY1=VALUE1 --variable KEY2=VALUE2`
    #[arg(long = "variable", value_name = "VARIABLE", value_parser = parse_key_val::<String, String>)]
    pub variables: Vec<(String, String)>,
}

impl CreateCommand {
    /// Run the creation of a node using a node configuration
    #[instrument(skip_all)]
    pub async fn run_config(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        debug!("Running node create with a node config");
        let mut node_config = self.get_node_config().await?;
        node_config.merge(&self)?;

        if self.foreground_args.foreground {
            node_config.run_foreground(ctx, &opts).await
        } else {
            node_config.run(ctx, &opts).await
        }
    }

    /// Try to read the self.name field as either:
    ///  - a URL to a configuration file
    ///  - a local path to a configuration file
    ///  - an inline configuration
    #[instrument(skip_all, fields(app.event.command.configuration_file))]
    pub async fn get_node_config(&self) -> miette::Result<NodeConfig> {
        let contents = match self.config_args.node_config.clone() {
            Some(contents) => contents,
            None => async_parse_path_or_url(&self.name).await?,
        };
        // Set environment variables from the cli command args
        // This needs to be done before parsing the configuration
        for (key, value) in &self.config_args.variables {
            if value.is_empty() {
                return Err(miette!("Empty value for variable '{key}'"));
            }
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
        for (key, value) in &cli_args.config_args.variables {
            if value.is_empty() {
                return Err(miette!("Empty value for variable '{key}'"));
            }
            std::env::set_var(key, value);
        }

        // Use a random name for the node if none has been specified
        if self.node.name.is_none() {
            self.node.name = Some(ArgValue::String(random_name()));
        }

        // Set the enrollment ticket from the cli command
        // overriding the one from the config file.
        if let Some(ticket) = &cli_args.config_args.enrollment_ticket {
            self.project_enroll.ticket = Some(ticket.hex_encoded()?);
        }

        // Merge the node arguments from the config with the cli command args.
        if self.node.skip_is_running_check.is_none() {
            self.node.skip_is_running_check = Some(ArgValue::Bool(cli_args.skip_is_running_check));
        }
        if self.node.foreground.is_none() {
            self.node.foreground = Some(ArgValue::Bool(cli_args.foreground_args.foreground));
        }
        if self.node.child_process.is_none() {
            self.node.exit_on_eof = Some(ArgValue::Bool(cli_args.foreground_args.child_process));
        }
        if self.node.exit_on_eof.is_none() {
            self.node.exit_on_eof = Some(ArgValue::Bool(cli_args.foreground_args.exit_on_eof));
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
        debug!("Running node config");
        // Parse then run commands
        for section in self.parse_commands()? {
            section.run(ctx, opts).await?
        }
        Ok(())
    }

    pub async fn run_foreground(
        self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        debug!("Running node config in foreground mode");
        // First, run the `project enroll` commands to prepare the identity and project data
        self.project_enroll
            .run_in_subprocess(opts.global_args.quiet)?
            .wait()
            .await
            .into_diagnostic()?;

        let node_name = self.node.name();

        // Next, run the 'node create' command
        let mut child = self.node.run_in_subprocess(opts.global_args.quiet)?;

        // Wait for the node to be up
        let is_up = {
            let node_name = node_name
                .as_ref()
                .expect("Node name should be set to the default value");
            let ctx = ctx.async_try_clone().await.into_diagnostic()?;
            let mut node =
                BackgroundNodeClient::create_to_node(&ctx, &opts.state, node_name).await?;
            is_node_up(&ctx, &mut node, true).await?
        };
        if !is_up {
            return Err(miette!("Node failed to start"));
        }

        // Run the other sections
        let other_sections: Vec<ParsedCommands> = vec![
            self.policies.into_parsed_commands()?.into(),
            self.relays.into_parsed_commands(&node_name)?.into(),
            self.tcp_inlets.into_parsed_commands(&node_name)?.into(),
            self.tcp_outlets.into_parsed_commands(&node_name)?.into(),
            self.kafka_inlet.into_parsed_commands(&node_name)?.into(),
            self.kafka_outlet.into_parsed_commands(&node_name)?.into(),
        ];
        for cmds in other_sections {
            cmds.run(ctx, opts).await?;
        }

        // Block on the foreground node
        child.wait().await.into_diagnostic()?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }

    /// Build commands and return validation errors if any
    fn parse_commands(self) -> miette::Result<Vec<ParsedCommands>> {
        let node_name = self.node.name();
        Ok(vec![
            self.project_enroll.into_parsed_commands()?.into(),
            self.node.into_parsed_commands()?.into(),
            self.policies.into_parsed_commands()?.into(),
            self.relays.into_parsed_commands(&node_name)?.into(),
            self.tcp_inlets.into_parsed_commands(&node_name)?.into(),
            self.tcp_outlets.into_parsed_commands(&node_name)?.into(),
            self.kafka_inlet.into_parsed_commands(&node_name)?.into(),
            self.kafka_outlet.into_parsed_commands(&node_name)?.into(),
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
            config_args: ConfigArgs {
                enrollment_ticket: Some(enrollment_ticket.clone()),
                ..Default::default()
            },
            ..Default::default()
        };

        // No node config, cli args should be used
        let mut config = NodeConfig::parse("").unwrap();
        config.merge(&cli_args).unwrap();
        let node = config.node.into_parsed_commands().unwrap().pop().unwrap();
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
        let node = config.node.into_parsed_commands().unwrap().pop().unwrap();
        assert_eq!(node.name, "n1");
        assert_eq!(node.tcp_listener_address, "127.0.0.1:5555".to_string());
        assert_eq!(
            config.project_enroll.ticket,
            Some(enrollment_ticket_hex.clone())
        );
    }
}
