use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};

use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::KeyValue;
use tracing::instrument;

use ockam_api::cli_state::random_name;
use ockam_api::logs::TracingGuard;
use ockam_core::AsyncTryClone;
use ockam_node::Context;

use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::util::api::TrustOpts;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::{async_cmd, local_cmd};
use crate::{docs, CommandGlobalOpts, Result};
use ockam_node::{opentelemetry_context_parser, OpenTelemetryContext};

pub mod background;
pub mod foreground;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the node.
    #[arg(hide_default_value = true, default_value_t = random_name())]
    pub node_name: String,

    /// Run the node in foreground.
    #[arg(display_order = 900, long, short)]
    pub foreground: bool,

    /// Skip the check if such node is already running.
    /// Useful for kubernetes when the pid is the same on each run.
    #[arg(long, short, value_name = "BOOL", default_value_t = false)]
    skip_is_running_check: bool,

    /// Watch stdin for EOF
    #[arg(display_order = 900, long = "exit-on-eof", short)]
    pub exit_on_eof: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// `node create` started a child process to run this node in foreground.
    #[arg(long, hide = true)]
    pub child_process: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(long, hide = true, value_parser = parse_launch_config)]
    pub launch_config: Option<Config>,

    /// Name of the Identity that the node will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,

    #[command(flatten)]
    pub trust_opts: TrustOpts,

    /// Serialized opentelemetry context
    #[arg(long, hide = true, value_parser = opentelemetry_context_parser)]
    pub opentelemetry_context: Option<OpenTelemetryContext>,
}

impl Default for CreateCommand {
    fn default() -> Self {
        let node_manager_defaults = NodeManagerDefaults::default();
        Self {
            skip_is_running_check: false,
            node_name: random_name(),
            exit_on_eof: false,
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            foreground: false,
            child_process: false,
            launch_config: None,
            identity: None,
            trust_opts: node_manager_defaults.trust_opts,
            opentelemetry_context: None,
        }
    }
}

impl CreateCommand {
    #[instrument(skip_all)]
    pub fn run(
        self,
        opts: CommandGlobalOpts,
        tracing_guard: Option<Arc<TracingGuard>>,
    ) -> miette::Result<()> {
        if self.foreground {
            if self.child_process {
                opentelemetry::Context::current()
                    .span()
                    .set_attribute(KeyValue::new("background", "true"));
            }
            local_cmd(embedded_node_that_is_not_stopped(
                opts.rt.clone(),
                |ctx| async move { self.foreground_mode(&ctx, opts, tracing_guard).await },
            ))
        } else {
            async_cmd(&self.name(), opts.clone(), |ctx| async move {
                self.background_mode(&ctx, opts).await
            })
        }
    }
    pub fn name(&self) -> String {
        if self.child_process {
            "create background node".into()
        } else {
            "create node".into()
        }
    }

    pub async fn async_run(
        self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        tracing_guard: Option<TracingGuard>,
    ) -> miette::Result<()> {
        let ctx = ctx.async_try_clone().await.into_diagnostic()?;
        if self.foreground {
            self.foreground_mode(&ctx, opts, tracing_guard.map(Arc::new))
                .await
        } else {
            self.background_mode(&ctx, opts).await
        }
    }

    fn logging_to_file(&self) -> bool {
        // Background nodes will spawn a foreground node in a child process.
        // In that case, the child process will log to files.
        if self.child_process {
            true
        }
        // The main process will log to stdout only if it's a foreground node.
        else {
            !self.foreground
        }
    }

    pub fn logging_to_stdout(&self) -> bool {
        !self.logging_to_file()
    }

    pub async fn guard_node_is_not_already_running(
        &self,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if !self.child_process {
            if let Ok(node) = opts.state.get_node(&self.node_name).await {
                if node.is_running() {
                    return Err(miette!("Node {} is already running", &self.node_name));
                }
            }
        }
        Ok(())
    }
}

pub fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path)
                .into_diagnostic()
                .wrap_err(miette!("Not a valid path"))?;
            Config::read(path)
        }
    }
}
