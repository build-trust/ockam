use crate::node::CreateCommand;
use crate::run::parser::building_blocks::ArgValue;
use crate::run::parser::config::ConfigParser;
use crate::run::parser::resource::*;
use crate::run::parser::Version;
use crate::value_parsers::async_parse_path_or_url;
use crate::CommandGlobalOpts;
use ockam_api::random_name;
use ockam_node::Context;
use serde::{Deserialize, Serialize};

impl CreateCommand {
    pub async fn run_config(self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        let contents = async_parse_path_or_url(&self.name).await?;
        let mut config = NodeConfig::new(&contents)?;
        let node_name = config.merge(self)?;
        config.run(ctx, opts.clone(), &node_name).await?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
    pub relays: Relays,
}

impl ConfigParser<'_> for NodeConfig {}

impl NodeConfig {
    fn new(contents: &str) -> miette::Result<Self> {
        Self::parse(&Self::resolve(contents)?)
    }

    /// Merge the arguments of the node defined in the config with the arguments from the
    /// "create" command, giving precedence to the config values.
    fn merge(&mut self, cli_args: CreateCommand) -> miette::Result<String> {
        // Set environment variables from the cli command
        // overriding the duplicates the config file.
        for (key, value) in &cli_args.variables {
            std::env::set_var(key, value);
        }

        // Set the enrollment ticket from the cli command
        // overriding the one from the config file.
        if let Some(ticket) = &cli_args.enrollment_ticket {
            self.project_enroll.ticket = Some(ticket.hex_encoded()?);
        }

        // Merge the node arguments from the config with the cli command args.
        if self.node.name.is_none() {
            self.node.name = Some(ArgValue::String(random_name()));
        }
        if self.node.skip_is_running_check.is_none() {
            self.node.skip_is_running_check = Some(ArgValue::Bool(cli_args.skip_is_running_check));
        }
        if self.node.exit_on_eof.is_none() {
            self.node.exit_on_eof = Some(ArgValue::Bool(cli_args.exit_on_eof));
        }
        if self.node.tcp_listener_address.is_none() {
            self.node.tcp_listener_address = Some(ArgValue::String(cli_args.tcp_listener_address));
        }
        if self.node.identity.is_none() {
            self.node.identity = cli_args.identity.map(ArgValue::String);
        }
        if self.node.project.is_none() {
            self.node.project = cli_args.trust_opts.project_name.map(ArgValue::String);
        }

        let node_name = self.node.name.as_ref().unwrap().to_string();
        Ok(node_name)
    }

    pub async fn run(
        self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        node_name: &str,
    ) -> miette::Result<()> {
        let overrides = &ValuesOverrides::default().with_override_node_name(node_name);

        // Build commands and return validation errors before running any command.
        let commands = vec![
            self.project_enroll.parse_commands(overrides)?,
            self.node.parse_commands(overrides)?,
            self.relays.parse_commands(overrides)?,
            self.policies.parse_commands(overrides)?,
            self.tcp_outlets.parse_commands(overrides)?,
            self.tcp_inlets.parse_commands(overrides)?,
        ];

        // Run commands
        for cmd in commands {
            cmd.run(ctx, &opts).await?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_api::authenticator::one_time_code::OneTimeCode;
    use ockam_api::EnrollmentTicket;

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
        let node_name = config.merge(cli_args.clone()).unwrap();
        let node = config
            .node
            .parse_commands(&ValuesOverrides::default())
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(node_name, node.name);
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
        let node_name = config.merge(cli_args).unwrap();
        let node = config
            .node
            .parse_commands(&ValuesOverrides::default())
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(node_name, node.name);
        assert_eq!(node_name, "n1");
        assert_eq!(node.tcp_listener_address, "127.0.0.1:5555".to_string());
        assert_eq!(
            config.project_enroll.ticket,
            Some(enrollment_ticket_hex.clone())
        );
    }
}
