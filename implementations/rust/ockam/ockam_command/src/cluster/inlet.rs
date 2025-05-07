use crate::cluster::common_args::{ClusterArg, HttpApiArgs, ZoneNameOrConfigArg};
use crate::cluster::utils::get_api_client;
use crate::node::config::ConfigArgs;
use crate::node::node_callback::NodeCallback;
use crate::node::util::wait_for_node_callback_future;
use crate::node_command::InMemoryNodeCommand;
use crate::tcp::inlet::create::tcp_inlet_default_from_addr;
use crate::util::foreground_args::ForegroundArgs;
use crate::util::parsers::hostname_parser;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, WrapErr};
use ockam::transport::SchemeHostnamePort;
use ockam_abac::PolicyExpression;
use ockam_api::cli_state::OCKAM_HOME;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::ai_platform::api::AiPlatformApi;
use ockam_api::CliState;
use ockam_node::Context;
use std::collections::BTreeMap;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/inlet/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/inlet/after_long_help.txt");

/// Connect to a service provided by an Ockam AI Agent
#[derive(Clone, Debug, Args, Default)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct InletCommand {
    #[command(flatten)]
    pub cluster: ClusterArg,

    #[command(flatten)]
    pub zone: ZoneNameOrConfigArg,

    /// References the name of TCP Outlet created in the Zone and the Relay name.
    #[arg(long)]
    pub pod: String,

    // == Node Options ==
    #[arg(long, env = "ENROLLMENT_TICKET", value_name = "ENROLLMENT TICKET")]
    #[arg(help = docs::about("\
    A path, URL or inlined hex-encoded enrollment ticket to use for the Ockam Identity associated to this node. \
    When passed, the identity will be given a project membership credential. \
    Check the `project ticket` command for more information about enrollment tickets.
    "))]
    pub enrollment_ticket: Option<String>,

    #[arg(long)]
    pub background: bool,

    /// Disable the Ctrl-C handler.
    #[arg(long)]
    pub no_ctrlc_handler: bool,

    #[command(flatten)]
    pub http_api: HttpApiArgs,

    // == TCP Inlet Options ==
    /// Address on which to accept TCP connections, in the format `<scheme>://<host>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default host is `127.0.0.1`.
    /// If the argument is not set, a random port will be used on the default address `tcp://127.0.0.1`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = tcp_inlet_default_from_addr(), value_parser = hostname_parser
    )]
    pub from: SchemeHostnamePort,

    /// Name of the TCP Outlet service to connect to.
    #[arg(long, id = "ROUTE")]
    pub to: Option<String>,

    #[arg(help = docs::about("\
     Policy expression that will be used for access control to the TCP Inlet. \
     If you don't provide it, the policy set for the \"tcp-inlet\" resource type will be used. \
     \n\nYou can check the fallback policy with `ockam policy show --resource-type tcp-inlet`."))]
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 900,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,
}

#[derive(Clone)]
struct InletNodeCommand {
    opts: CommandGlobalOpts,
    command: InletCommand,
}

#[async_trait]
impl InMemoryNodeCommand for InletNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let use_http_api = self.command.http_api.use_http_api();
        let api_client = get_api_client(&node, use_http_api).await?;
        let cluster = self.command.cluster.get_cluster(ctx, &node).await?;
        let zone_name = self.command.zone.zone_name()?;
        let enrollment_ticket = self
            .get_enrollment_ticket(ctx, &*api_client, &cluster, &zone_name)
            .await?;
        let relay_name = format!("{}-{}-{}", cluster, zone_name, self.command.pod);
        let outlet_name = self.command.to.as_ref().unwrap_or(&self.command.pod);
        let mut node_config = serde_json::json!({
            "tcp-inlet": {
                "from": self.command.from.to_string(),
                "to": outlet_name,
                "via": relay_name
            }
        });
        if let Some(allow) = &self.command.allow {
            node_config["tcp-inlet"]["allow"] = allow.to_string().into();
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
                enrollment_ticket: Some(enrollment_ticket),
                ..Default::default()
            },
            foreground_args: ForegroundArgs {
                foreground: true,
                no_ctrlc_handler: self.command.no_ctrlc_handler,
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
impl Command for InletCommand {
    const NAME: &'static str = "cluster inlet";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = InletNodeCommand {
            opts: opts.clone(),
            command: self.clone(),
        };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}

impl InletNodeCommand {
    async fn get_enrollment_ticket(
        &self,
        ctx: &Context,
        api_client: &(dyn AiPlatformApi + Send + Sync + 'static),
        cluster: &str,
        zone_name: &str,
    ) -> Result<String> {
        if let Some(t) = &self.command.enrollment_ticket {
            return Ok(t.clone());
        }
        api_client
            .create_enrollment_token(ctx, cluster, zone_name, BTreeMap::default(), None)
            .await.wrap_err("Failed to generate an enrollment ticket for the inlet. Please provide one with the --enrollment-ticket argument")
    }
}
