use std::{path::PathBuf, str::FromStr};

use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};

use ockam::identity::Identity;
use ockam_api::cli_state::random_name;

use crate::node::create::background::background_mode;
use crate::node::create::foreground::foreground_mode;
use crate::node::util::NodeManagerDefaults;
use crate::service::config::Config;
use crate::util::api::TrustContextOpts;
use crate::util::embedded_node_that_is_not_stopped;
use crate::util::{local_cmd, node_rpc};
use crate::{docs, CommandGlobalOpts, Result};

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

    #[arg(long, group = "trusted")]
    pub trusted_identities: Option<String>,
    #[arg(long, group = "trusted")]
    pub trusted_identities_file: Option<PathBuf>,
    #[arg(long, group = "trusted")]
    pub reload_from_trusted_identities_file: Option<PathBuf>,

    /// Name of the Identity that the node will use
    #[arg(long = "identity", value_name = "IDENTITY_NAME")]
    identity: Option<String>,

    /// Hex encoded Identity
    #[arg(long, value_name = "IDENTITY")]
    authority_identity: Option<String>,

    #[arg(long = "credential", value_name = "CREDENTIAL_NAME")]
    pub credential: Option<String>,

    #[command(flatten)]
    pub trust_context_opts: TrustContextOpts,
}

impl Default for CreateCommand {
    fn default() -> Self {
        let node_manager_defaults = NodeManagerDefaults::default();
        Self {
            node_name: random_name(),
            exit_on_eof: false,
            tcp_listener_address: node_manager_defaults.tcp_listener_address,
            foreground: false,
            child_process: false,
            launch_config: None,
            identity: None,
            authority_identity: None,
            trusted_identities: None,
            trusted_identities_file: None,
            reload_from_trusted_identities_file: None,
            credential: None,
            trust_context_opts: node_manager_defaults.trust_context_opts,
        }
    }
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        if self.foreground {
            local_cmd(embedded_node_that_is_not_stopped(
                foreground_mode,
                (opts, self),
            ));
        } else {
            node_rpc(background_mode, (opts, self))
        }
    }

    async fn authority_identity(&self) -> Result<Option<Identity>> {
        match &self.authority_identity {
            Some(i) => Ok(Some(Identity::create(i).await.into_diagnostic()?)),
            None => Ok(None),
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

pub async fn guard_node_is_not_already_running(
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
) -> miette::Result<()> {
    if !cmd.child_process {
        if let Ok(node) = opts.state.get_node(&cmd.node_name).await {
            if node.is_running() {
                return Err(miette!("Node {} is already running", &cmd.node_name));
            }
        }
    }
    Ok(())
}
