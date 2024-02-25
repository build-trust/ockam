use std::str::FromStr;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::info;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::service::relay::Relays;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::CliState;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{async_cmd, colorize_connection_status, process_nodes_multiaddr};
use crate::{docs, fmt_log, fmt_ok, CommandGlobalOpts, Error, Result};
use crate::{node::util::initialize_default_node, terminal::color_primary};

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

    /// Whether the relay will be used to relay messages at a project.
    /// By default, this information will be inferred from the `--at` argument.
    #[arg(long)]
    project_relay: bool,
}

pub fn default_at_addr() -> String {
    "/project/$DEFAULT_PROJECT_NAME".to_string()
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "create relay".into()
    }

    pub async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;
        let at = cmd.at();
        let alias = cmd.relay_name();

        opts.terminal.write_line(&fmt_log!("Creating Relay...\n"))?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.to).await?;
        let get_relay_info = async {
            let relay_info = {
                if at.starts_with(Project::CODE) && cmd.authorized.is_some() {
                    return Err(miette!(
                        "--authorized can not be used with project addresses"
                    ))?;
                };
                info!("creating a relay at {} to {}", at, node.node_name());
                node.create_relay(
                    ctx,
                    &at,
                    alias.clone(),
                    cmd.authorized,
                    Some(cmd.relay_address.unwrap_or(alias)),
                    !cmd.project_relay,
                )
                .await?
            };
            *is_finished.lock().await = true;
            Ok(relay_info)
        };

        let output_messages = vec![
            format!(
                "Creating relay relay service at {}...",
                &at.to_string().color(OckamColor::PrimaryResource.color())
            ),
            format!(
                "Setting up receiving relay mailbox on node {}...",
                &node.node_name().color(OckamColor::PrimaryResource.color())
            ),
        ];
        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (relay, _) = try_join!(get_relay_info, progress_output)?;

        let plain = {
            // `remote_address` in the project is relaying to worker at address `worker_address` on that node.
            let remote_address = relay
                .remote_address_ma()
                .into_diagnostic()?
                .map(|x| x.to_string())
                .unwrap_or("N/A".into());
            let worker_address = relay
                .worker_address_ma()
                .into_diagnostic()?
                .map(|x| x.to_string())
                .unwrap_or("N/A".into());
            let from = color_primary(format!("{}{}", &at, remote_address));
            let to = color_primary(format!("/node/{}{}", &node.node_name(), worker_address));
            fmt_ok!("Now relaying messages from {from} → {to}")
        };

        let machine = relay
            .remote_address_ma()
            .into_diagnostic()?
            .map(|x| x.to_string())
            .unwrap_or("N/A".into());

        let json = serde_json::to_string_pretty(&relay).into_diagnostic()?;
        opts.terminal
            .stdout()
            .plain(plain)
            .machine(machine)
            .json(json)
            .write_line()?;

        Ok(())
    }

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
        let relay_name = Self::parse_arg_relay_name(self.relay_name, !self.project_relay)?;
        self.at = at.to_string();
        self.relay_name = relay_name;
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

    fn parse_arg_relay_name(relay_name: impl Into<String>, at_rust_node: bool) -> Result<String> {
        let relay_name = relay_name.into();
        if at_rust_node {
            Ok(format!("forward_to_{relay_name}"))
        } else {
            Ok(relay_name)
        }
    }
}

impl Output for RelayInfo {
    fn output(&self) -> Result<String> {
        Ok(r#"
Relay:
    "#
        .to_owned()
            + self.list_output()?.as_str())
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Alias: {alias}
Status: {connection_status}
Remote Address: {remote_address}"#,
            alias = self.alias().color(OckamColor::PrimaryResource.color()),
            connection_status = colorize_connection_status(self.connection_status()),
            remote_address = self
                .remote_address_ma()?
                .map(|x| x.to_string())
                .unwrap_or("N/A".into())
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use ockam_api::nodes::InMemoryNode;

    use super::*;

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

        ctx.stop().await.unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_parse_arg_relay_name() {
        // `--at` is a local route
        let res = CreateCommand::parse_arg_relay_name("relay", true).unwrap();
        assert_eq!(res, "forward_to_relay");

        // `--at` is a remote route
        let res = CreateCommand::parse_arg_relay_name("relay", false).unwrap();
        assert_eq!(res, "relay");
    }
}
