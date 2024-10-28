use async_trait::async_trait;
use std::str::FromStr;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tracing::info;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::colors::color_primary;

use ockam_api::nodes::models::relay::ReturnTiming;
use ockam_api::nodes::service::relay::Relays;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_ok, fmt_warn, CliState, ConnectionStatus};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::node::util::initialize_default_node;
use crate::shared_args::RetryOpts;
use crate::util::{print_deprecated_flag_warning, process_nodes_multiaddr};
use crate::{docs, Command, CommandGlobalOpts, Error, Result};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");

/// Create a Relay
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = false,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the relay. If not provided, 'default' will be used.
    #[arg(hide_default_value = true, default_value = "default")]
    pub relay_name: String,

    /// Node for which to create the relay
    #[arg(long, id = "NODE_NAME", value_parser = extract_address_value)]
    pub to: Option<String>,

    /// Route to the node at which to create the relay
    #[arg(long, id = "ROUTE", default_value_t = default_at_addr())]
    pub at: String,

    /// Authorized identity for secure channel connection
    #[arg(long, id = "AUTHORIZED")]
    pub authorized: Option<Identifier>,

    /// Relay address to use. By default, inherits the relay name.
    #[arg(long)]
    relay_address: Option<String>,

    /// [DEPRECATED] Whether the relay will be used to relay messages at a project.
    /// By default, this information will be inferred from the `--at` argument.
    #[arg(long)]
    project_relay: bool,

    /// Create the Relay without waiting for the connection to be established
    #[arg(long, default_value = "false")]
    pub no_connection_wait: bool,

    #[command(flatten)]
    retry_opts: RetryOpts,
}

pub fn default_at_addr() -> String {
    "/project/$DEFAULT_PROJECT_NAME".to_string()
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "relay create";

    fn retry_opts(&self) -> Option<RetryOpts> {
        Some(self.retry_opts.clone())
    }

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        if self.project_relay {
            print_deprecated_flag_warning(&opts, "--project-relay")?;
        }

        initialize_default_node(ctx, &opts).await?;

        let cmd = self.parse_args(&opts).await?;
        let at = cmd.at();
        let alias = cmd.relay_name();
        let return_timing = cmd.return_timing();

        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.to).await?;
        let relay_info = {
            if at.starts_with(Project::CODE) && cmd.authorized.is_some() {
                return Err(miette!(
                    "--authorized can not be used with project addresses"
                ))?;
            };
            info!("creating a relay at {} to {}", at, node.node_name());
            let pb = opts.terminal.progress_bar();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating relay at {}...",
                    color_primary(at.to_string())
                ));
            }
            node.create_relay(
                ctx,
                &at,
                alias.clone(),
                cmd.authorized,
                Some(cmd.relay_address.unwrap_or(alias)),
                return_timing.clone(),
            )
            .await
            .map_err(Error::Retry)?
        };

        match return_timing {
            ReturnTiming::Immediately => {
                let plain = {
                    let from = color_primary(&at);
                    let to = color_primary(format!("/node/{}", &node.node_name()));

                    fmt_ok!("Relay will be created automatically from {from} → {to} as soon as a connection can be established.")
                };

                opts.terminal
                    .stdout()
                    .plain(plain)
                    .json_obj(relay_info)?
                    .write_line()?;
            }
            ReturnTiming::AfterConnection => {
                if relay_info.connection_status() == ConnectionStatus::Up {
                    let invalid_relay_error_msg =
                        "The Orchestrator returned an invalid relay address. Try creating a new one.";

                    let remote_address = relay_info
                        .remote_address_ma()
                        .into_diagnostic()?
                        .ok_or(miette!(invalid_relay_error_msg))?;
                    let worker_address = relay_info
                        .worker_address_ma()
                        .into_diagnostic()?
                        .ok_or(miette!(invalid_relay_error_msg))?;

                    let plain = {
                        // `remote_address` in the project is relaying to worker at address `worker_address` on that node.
                        let from = color_primary(format!("{}{}", &at, remote_address));
                        let to =
                            color_primary(format!("/node/{}{}", &node.node_name(), worker_address));

                        fmt_ok!("Now relaying messages from {from} → {to}")
                    };

                    opts.terminal
                        .stdout()
                        .plain(plain)
                        .machine(remote_address.to_string())
                        .json_obj(relay_info)?
                        .write_line()?;
                } else {
                    let plain = {
                        let from = color_primary(&at);
                        let to = color_primary(format!("/node/{}", &node.node_name()));

                        fmt_warn!("A relay was created at {to} but failed to connect to {from}\n")
                            + &fmt_info!("It will retry to connect automatically")
                    };

                    opts.terminal
                        .stdout()
                        .plain(plain)
                        .json_obj(relay_info)?
                        .write_line()?;
                }
            }
        }

        Ok(())
    }
}

impl CreateCommand {
    fn at(&self) -> MultiAddr {
        MultiAddr::from_str(&self.at).unwrap()
    }

    fn relay_name(&self) -> String {
        self.relay_name.clone()
    }

    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> Result<Self> {
        let default_project_name = &opts
            .state
            .projects()
            .get_default_project()
            .await
            .ok()
            .map(|p| p.name().to_string());
        let at = Self::parse_arg_at(&opts.state, self.at, default_project_name.as_deref()).await?;
        self.project_relay |= at.starts_with(Project::CODE);
        self.at = at.to_string();
        Ok(self)
    }

    async fn parse_arg_at(
        state: &CliState,
        at: impl Into<String>,
        default_project_name: Option<&str>,
    ) -> Result<MultiAddr> {
        let mut at = at.into();
        // The address is a node name
        if !at.contains('/') {
            at = format!("/node/{at}");
        }
        // The address is a project, parse it.
        else if at.starts_with("/project/") {
            let project_name = default_project_name.ok_or(Error::NotEnrolled)?;
            at = at.replace("$DEFAULT_PROJECT_NAME", project_name);
        }
        let ma = MultiAddr::from_str(&at).map_err(|_| Error::arg_validation("at", at, None))?;
        process_nodes_multiaddr(&ma, state).await
    }

    fn return_timing(&self) -> ReturnTiming {
        if self.no_connection_wait {
            ReturnTiming::Immediately
        } else {
            ReturnTiming::AfterConnection
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::parser::resource::utils::parse_cmd_from_args;
    use ockam_api::nodes::InMemoryNode;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }

    #[ockam_macros::test(crate = "ockam")]
    async fn test_parse_arg_at(ctx: &mut Context) -> ockam::Result<()> {
        let state = CliState::test().await?;
        let default_project_name = Some("p1");

        // Invalid values
        CreateCommand::parse_arg_at(&state, "/alice/service", default_project_name)
            .await
            .expect_err("Invalid protocol");
        CreateCommand::parse_arg_at(&state, "my/project", default_project_name)
            .await
            .expect_err("Invalid protocol");
        CreateCommand::parse_arg_at(&state, "alice", default_project_name)
            .await
            .expect_err("Node doesn't exist");

        // The placeholder is replaced when using the arg's default value
        let res = CreateCommand::parse_arg_at(&state, default_at_addr(), default_project_name)
            .await
            .unwrap()
            .to_string();
        assert_eq!(res, "/project/p1");

        // The user provides a full project route
        let addr = "/project/p1";
        let res = CreateCommand::parse_arg_at(&state, addr, default_project_name)
            .await
            .unwrap()
            .to_string();
        assert_eq!(res, addr);

        // The user provides the name of a node
        let node = InMemoryNode::start(ctx, &state).await.unwrap();
        let res = CreateCommand::parse_arg_at(&state, &node.node_name(), default_project_name)
            .await
            .unwrap()
            .to_string();
        assert!(res.contains("/ip4/127.0.0.1/tcp/"));

        Ok(())
    }
}
