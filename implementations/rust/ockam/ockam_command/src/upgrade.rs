use crate::util::installer::upgrade;
use clap::{crate_version, Args};
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_core::env::get_env_with_default;
use serde::Deserialize;
use std::env;
use tokio::runtime::Builder;

use crate::{
    fmt_info, fmt_ok, node::util::spawn_node, terminal::ConfirmResult, util::node_rpc,
    CommandGlobalOpts, Result,
};

#[derive(Deserialize, Debug)]
struct UpgradeFile {
    upgrade_message: Option<String>,
    upgrade_message_macos: Option<String>,
}

pub fn check_if_an_upgrade_is_available() {
    if !upgrade_check_is_disabled() {
        // check if a new version has been released
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(check());
    }
}

async fn check() {
    let url = format!(
        "https://github.com/build-trust/ockam/releases/download/ockam_v{}/upgrade.json",
        crate_version!()
    );
    let resp = reqwest::get(url).await;

    if let Ok(r) = resp {
        if let Ok(upgrade) = r.json::<UpgradeFile>().await {
            if let Some(message) = upgrade.upgrade_message {
                eprintln!("\n{}", message.yellow());

                if cfg!(target_os = "macos") {
                    if let Some(message) = upgrade.upgrade_message_macos {
                        eprintln!("\n{}", message.yellow());
                    }
                }

                eprintln!();
            }
        }
    }
}

fn upgrade_check_is_disabled() -> bool {
    get_env_with_default("OCKAM_DISABLE_UPGRADE_CHECK", false).unwrap_or(false)
}

#[derive(Deserialize, Debug)]
struct LatestRelease {
    name: String,
}

impl LatestRelease {
    fn version(&self) -> Result<&str> {
        let result = self.name.split_once('v');
        match result {
            Some((_, version)) => Ok(version),
            None => Err(miette!("Failed to get latest release version").into()),
        }
    }
}

async fn get_latest_release_version() -> Result<LatestRelease> {
    let url = "https://api.github.com/repos/build-trust/ockam/releases/latest";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Ockam Command")
        .send()
        .await;
    if let Ok(r) = resp {
        if let Ok(release) = r.json::<LatestRelease>().await {
            return Ok(release);
        }
    }
    Err(miette!("Failed to get latest release").into())
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
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, UpgradeCommand),
) -> miette::Result<()> {
    let latest_release = get_latest_release_version().await?;
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

    if cmd.check {
        opts.terminal
            .stdout()
            .plain(fmt_info!(
                "A new version of ockam is available: {}. Current ockam version: {}",
                latest_version,
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

    upgrade_ockam(latest_version, &opts)?;
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

fn start_nodes(stopped_nodes_names: &[String], opts: &CommandGlobalOpts) -> miette::Result<()> {
    opts.terminal
        .write_line(fmt_info!("Restarting all stopped nodes"))?;
    for node_name in stopped_nodes_names.iter() {
        let node_state = opts.state.nodes.get(node_name)?;
        node_state.kill_process(false)?;
        let node_setup = node_state.config().setup();
        let x = &node_setup.default_tcp_listener()?.addr.to_string();
        spawn_node(
            opts,
            node_setup.verbose,
            node_state.name(),
            x,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?;
        opts.terminal
            .write_line(fmt_ok!("Restarted node {}", node_state.name()))?;
    }
    Ok(())
}

fn upgrade_ockam(latest_version: &str, opts: &CommandGlobalOpts) -> miette::Result<()> {
    let stopped_nodes_names = stop_all_running_nodes(opts)?;
    let result = upgrade(latest_version);
    // Try to restart nodes even if upgrade failed
    start_nodes(&stopped_nodes_names, opts)?;
    result
}
