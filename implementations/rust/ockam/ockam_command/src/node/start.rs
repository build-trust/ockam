use anyhow::{anyhow, Context as _};
use clap::Args;
use nix::unistd::Pid;
use rand::prelude::random;

use ockam::TcpTransport;

use crate::node::show::print_query_status;
use crate::node::util::run::CommandsRunner;
use crate::util::{node_rpc, RpcBuilder};
use crate::{
    help,
    node::HELP_DETAIL,
    util::{exitcode, startup::spawn_node},
    CommandGlobalOpts,
};

/// Start Nodes
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
pub struct StartCommand {
    /// Name of the node.
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    node_name: String,
}

impl StartCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(
    ctx: ockam::Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> crate::Result<()> {
    let cfg = &opts.config;
    let node_name = &cmd.node_name;
    let cfg_node = cfg.get_node(node_name)?;

    // First we check whether a PID was registered and if it is still alive.
    if let Some(pid) = cfg_node.pid() {
        // Note: On CI machines where <defunct> processes can occur,
        // the below `kill 0 pid` can imply a killed process is okay.
        let res = nix::sys::signal::kill(Pid::from_raw(pid), None);
        if res.is_ok() {
            return Err(crate::Error::new(
                exitcode::IOERR,
                anyhow!(
                    "Node '{}' already appears to be running as PID {}",
                    node_name,
                    pid
                ),
            ));
        }
    }

    // Restart node
    restart_background_node(&opts, &cmd).await?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    print_query_status(&mut rpc, cfg_node.port(), node_name, true).await?;

    // Run startup commands
    if let Ok(cfg) = cfg.node(&cmd.node_name) {
        CommandsRunner::run_node_startup(cfg.commands().config_path())
            .context("Failed to startup commands")?;
    }

    Ok(())
}

async fn restart_background_node(
    opts: &CommandGlobalOpts,
    cmd: &StartCommand,
) -> crate::Result<()> {
    let cfg = &opts.config;
    let cfg_node = cfg.get_node(&cmd.node_name)?;

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    spawn_node(
        &opts.config,
        cfg_node.verbose(),           // Previously user-chosen verbosity level
        true,                         // skip-defaults because the node already exists
        false,                        // Default value. TODO: implement persistence of this option
        false,                        // Default value. TODO: implement persistence of this option
        cfg_node.name(),              // The selected node name
        &cfg_node.addr().to_string(), // The selected node api address
        None,                         // No project information available
    )?;

    Ok(())
}
