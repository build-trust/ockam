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
use ockam_api::nodes::BackgroundNode;
use ockam_api::{is_local_node, CliState};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::node::util::initialize_default_node;
use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{node_rpc, process_nodes_multiaddr};
use crate::{display_parse_logs, fmt_ok, CommandGlobalOpts};
use crate::{docs, fmt_log, Error, Result};

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
    /// Name of the relay
    #[arg(hide_default_value = true, default_value = "default")]
    relay_name: String,

    /// Node for which to create the relay
    #[arg(long, id = "NODE_NAME", value_parser = extract_address_value)]
    to: Option<String>,

    /// Route to the node at which to create the relay
    #[arg(long, id = "ROUTE", default_value_t = default_at_addr())]
    at: String,

    /// Authorized identity for secure channel connection
    #[arg(long, id = "AUTHORIZED")]
    authorized: Option<Identifier>,
}

fn default_at_addr() -> String {
    "/project/$DEFAULT_PROJECT_NAME".to_string()
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
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
            .get_default_project()
            .await
            .ok()
            .map(|p| p.name());
        let at = Self::parse_arg_at(&opts.state, self.at, default_project_name.as_deref()).await?;
        let relay_name = Self::parse_arg_relay_name(self.relay_name, &at)?;
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

    fn parse_arg_relay_name(relay_name: impl Into<String>, at: &MultiAddr) -> Result<String> {
        let relay_name = relay_name.into();
        let at_rust_node = is_local_node(at)?;
        if at_rust_node {
            Ok(format!("forward_to_{relay_name}"))
        } else {
            Ok(relay_name)
        }
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    initialize_default_node(&ctx, &opts).await?;
    let cmd = cmd.parse_args(&opts).await?;
    let at = cmd.at();
    let alias = cmd.relay_name();

    opts.terminal.write_line(&fmt_log!("Creating Relay...\n"))?;
    display_parse_logs(&opts);
    let is_finished: Mutex<bool> = Mutex::new(false);

    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.to).await?;
    let get_relay_info = async {
        let relay_info = {
            if at.starts_with(Project::CODE) && cmd.authorized.is_some() {
                return Err(miette!("--authorized can not be used with project addresses").into());
            };
            info!("creating a relay at {} to {}", at, node.node_name());
            node.create_relay(&ctx, &at, Some(alias), cmd.authorized)
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
        let from = format!(
            "{}{}",
            &at,
            &relay.worker_address_ma().into_diagnostic()?.to_string()
        )
        .color(OckamColor::PrimaryResource.color());
        let to = format!(
            "/node/{}{}",
            &node.node_name(),
            &relay.remote_address_ma().into_diagnostic()?.to_string()
        )
        .color(OckamColor::PrimaryResource.color());
        fmt_ok!("Now relaying messages from {from} → {to}")
    };
    let machine = relay.remote_address_ma().into_diagnostic()?;
    let json = serde_json::to_string_pretty(&relay).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .machine(machine)
        .json(json)
        .write_line()?;

    Ok(())
}

impl Output for RelayInfo {
    fn output(&self) -> Result<String> {
        let output = format!(
            r#"
Relay {}:
    Route: {}
    Remote Address: {}
    Worker Address: {}
    Flow Control Id: {}"
"#,
            self.remote_address(),
            self.forwarding_route(),
            self.remote_address_ma()?,
            self.worker_address_ma()?,
            self.flow_control_id()
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("<none>".into())
        );

        Ok(output)
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Relay {}
Route {}"#,
            self.remote_address()
                .color(OckamColor::PrimaryResource.color()),
            self.forwarding_route()
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
        let at = MultiAddr::from_str("/node/alice").unwrap();
        let res = CreateCommand::parse_arg_relay_name("relay", &at).unwrap();
        assert_eq!(res, "forward_to_relay");

        // `--at` is a remote route
        let at = MultiAddr::from_str("/project/p1").unwrap();
        let res = CreateCommand::parse_arg_relay_name("relay", &at).unwrap();
        assert_eq!(res, "relay");
    }
}
