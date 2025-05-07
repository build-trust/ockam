use async_trait::async_trait;
use std::sync::Arc;

use crate::node::config::ConfigArgs;
use crate::node::node_callback::NodeCallback;
use crate::node::util::wait_for_node_callback_future;
use crate::node_command::InMemoryNodeCommand;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::parsers::hostname_parser;
use crate::{docs, Command, CommandGlobalOpts, Result};
use clap::Args;
use miette::IntoDiagnostic;
use ockam::transport::SchemeHostnamePort;
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::nodes::InMemoryNode;
use ockam_api::CliState;
use ockam_node::Context;

const LONG_ABOUT: &str = include_str!("./static/outlet/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/outlet/after_long_help.txt");

/// Connect to a service provided by an Ockam AI Agent
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct OutletCommand {
    // == Node Options ==
    #[arg(long, env = "ENROLLMENT_TICKET", value_name = "ENROLLMENT TICKET")]
    #[arg(help = docs::about("\
    A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node. \
    When passed, the identity will be given a project membership credential. \
    Check the `project ticket` command for more information about enrollment tickets.
    "))]
    pub enrollment_ticket: String,

    #[arg(long)]
    pub relay: String,

    #[arg(long)]
    pub background: bool,

    // == TCP Outlet Options ==
    /// Service address of your TCP Outlet, which is part of a route used in other commands.
    /// This unique address identifies the TCP Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    /// If not provided, `outlet` will be used, or a random address will be generated if `outlet` is taken.
    /// You will need this address when creating a TCP Inlet using `ockam tcp-inlet create`.
    #[arg(long, display_order = 902, id = "OUTLET_ADDRESS", value_parser = extract_address_value)]
    pub from: Option<String>,

    /// Network address where your application is listening to.
    /// Your TCP Outlet will forward raw TCP traffic to this destination.
    #[arg(long, id = "SOCKET_ADDRESS", display_order = 900, value_parser = hostname_parser)]
    pub to: SchemeHostnamePort,

    #[arg(help = docs::about("\
    Policy expression that will be used for access control to the TCP Outlet. \
    If you don't provide it, the policy set for the \"tcp-outlet\" resource type will be used. \
    \n\nYou can check the fallback policy with `ockam policy show --resource-type tcp-outlet`"))]
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 904,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,
}

#[derive(Clone)]
struct OutletNodeCommand {
    opts: CommandGlobalOpts,
    command: OutletCommand,
}

#[async_trait]
impl InMemoryNodeCommand for OutletNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let mut node_config = serde_json::json!({
            "relay": self.command.relay.clone(),
            "tcp-outlet": {
                "to": self.command.to.to_string(),
                }
        });
        if let Some(from) = &self.command.from {
            node_config["tcp-outlet"]["from"] = from.to_string().into();
        }
        if let Some(allow) = &self.command.allow {
            node_config["tcp-outlet"]["allow"] = allow.to_string().into();
        }
        let in_memory = true;
        let node_callback = if self.command.background {
            Some(NodeCallback::create().await?)
        } else {
            None
        };
        let node_cmd = crate::node::create::CreateCommand {
            name: node_config.to_string(),
            config_args: ConfigArgs {
                enrollment_ticket: Some(self.command.enrollment_ticket.clone()),
                ..Default::default()
            },
            foreground_args: ForegroundArgs {
                foreground: true,
                ..Default::default()
            },
            in_memory,
            tcp_callback_port: node_callback.as_ref().map(|n| n.callback_port()),
            ..Default::default()
        };
        let mut opts = self.opts.clone();
        let handle = tokio::spawn(async move {
            let tmp_dir = tempfile::tempdir().into_diagnostic()?;
            std::env::set_var(OCKAM_HOME, tmp_dir.path());
            opts.state = Arc::new(CliState::new(in_memory).await?);
            node_cmd.run(node.ctx(), opts).await?;
            Ok(())
        });
        if let Some(node_callback) = node_callback {
            wait_for_node_callback_future(handle, node_callback).await?;
        } else {
            handle.await.into_diagnostic()??;
        }
        Ok(())
    }
}

#[async_trait]
impl Command for OutletCommand {
    const NAME: &'static str = "cluster outlet";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = OutletNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
