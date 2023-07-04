use crate::util::{
    github::{check_upgrade_sync, get_latest_release_version_sync},
    installer::get_installer,
    local_cmd,
};
use clap::{crate_version, Args};
use colorful::Colorful;
use miette::miette;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_core::env::get_env_with_default;
use std::env;

use crate::{
    fmt_info, fmt_ok, node::util::spawn_node, terminal::ConfirmResult, CommandGlobalOpts, Result,
};

pub fn check_if_an_upgrade_is_available() {
    if upgrade_check_is_enabled() {
        check_upgrade_sync(); // check if a new version has been released
    }
}

fn upgrade_check_is_enabled() -> bool {
    !get_env_with_default("OCKAM_DISABLE_UPGRADE_CHECK", false).unwrap_or(false)
}

#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    subcommand_required = false,
    long_about = "Upgrade ockam to the latest version"
)]
pub struct UpgradeCommand {
    #[arg(long, short)]
    check: bool,
    #[arg(long, short)]
    yes: bool,
}

impl UpgradeCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        local_cmd(run_impl(opts, self));
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: UpgradeCommand) -> miette::Result<()> {
    let latest_release = get_latest_release_version_sync()?;
    let latest_version = latest_release.version()?;
    let current_version = crate_version!();

    if current_version == latest_version {
        opts.terminal
            .stdout()
            .plain(fmt_info!(
                "You are already running the latest version of ockam: {}",
                current_version
            ))
            .write_line()?;
        return Ok(());
    }

    opts.terminal.write_line(fmt_info!(
        "A new version of ockam is available: {}. Current ockam version: {}",
        latest_version,
        current_version
    ))?;

    if cmd.check {
        return Ok(());
    }

    if !cmd.yes {
        match opts.terminal.confirm(&fmt_info!(
            "This will upgrade ockam to the latest version. Are you sure?"
        ))? {
            ConfirmResult::Yes => {}
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm"));
            }
        }
    }
    opts.terminal.write_line(fmt_info!(
        "Upgrading ockam from {} to {}",
        current_version,
        latest_version
    ))?;

    upgrade_ockam(&opts)?;
    opts.terminal
        .stdout()
        .plain(fmt_ok!("Ockam upgraded to version {}", latest_version))
        .write_line()?;
    Ok(())
}

fn stop_all_running_nodes(opts: &CommandGlobalOpts) -> Result<Vec<String>> {
    opts.terminal
        .write_line(fmt_info!("Stopping all running nodes"))?;
    let nodes_states = opts.state.nodes.list()?;
    let mut stopped_nodes = Vec::new();
    for node_state in nodes_states.iter() {
        if node_state.is_running() {
            node_state.kill_process(false)?;
            opts.terminal
                .write_line(fmt_ok!("Stopped node {}", node_state.name()))?;
            stopped_nodes.push(node_state.name().to_string());
        }
    }
    Ok(stopped_nodes)
}

fn restart_nodes(stopped_nodes_names: &[String], opts: &CommandGlobalOpts) -> miette::Result<()> {
    opts.terminal
        .write_line(fmt_info!("Restarting all stopped nodes"))?;
    for node_name in stopped_nodes_names.iter() {
        let node_state = opts.state.nodes.get(node_name)?;
        node_state.kill_process(false)?;
        let node_setup = node_state.config().setup();
        spawn_node(
            &opts,
            &node_name,                                    // The selected node name
            &node_setup.api_transport()?.addr.to_string(), // The selected node api address
            None,                                          // No project information available
            None,                                          // No trusted identities
            None,                                          // "
            None,                                          // "
            None,                                          // Launch config
            None,                                          // Authority Identity
            None,                                          // Credential
            None,                                          // Trust Context
            None,                                          // Project Name
            node_setup.disable_file_logging,
        )?;
        opts.terminal
            .write_line(fmt_ok!("Restarted node {}", node_state.name()))?;
    }
    Ok(())
}

fn upgrade_ockam(opts: &CommandGlobalOpts) -> miette::Result<()> {
    let stopped_nodes_names = stop_all_running_nodes(opts)?;
    let installer = get_installer()?;
    let result = installer.upgrade();
    // Try to restart nodes even if upgrade failed
    restart_nodes(&stopped_nodes_names, opts)?;
    result
}
