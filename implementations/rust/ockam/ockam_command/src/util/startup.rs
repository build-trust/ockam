//! An automatic setup mechanism for composition snippets

#![allow(unused)]

use crate::exitcode;
use crate::util::OckamConfig;
use anyhow::Context;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use ockam_api::authenticator::direct::types::OneTimeCode;
use std::collections::VecDeque;
use std::io::Stdout;
use std::process::Stdio;
use std::{
    env::current_exe,
    fs::OpenOptions,
    path::{Path, PathBuf},
    process::Command,
};

/// Stop a node without deleting its state directory
pub fn stop(pid: i32, sigkill: bool) -> anyhow::Result<()> {
    signal::kill(
        Pid::from_raw(pid),
        if sigkill {
            Signal::SIGKILL
        } else {
            Signal::SIGTERM
        },
    )
    .context(format!("Failed to kill process with PID {pid}"))?;
    Ok(())
}

/// A utility function to spawn a new node into foreground mode
///
/// This function is used by `ockam node create` as well as `ockam
/// node start`, which attempts to re-use an existing node config
#[allow(clippy::too_many_arguments)]
pub fn spawn_node(
    cfg: &OckamConfig,
    verbose: u8,
    skip_defaults: bool,
    no_shared_identity: bool,
    enable_credential_checks: bool,
    name: &str,
    address: &str,
    project: Option<&Path>,
    invite: Option<&OneTimeCode>,
) -> crate::Result<()> {
    // On systems with non-obvious path setups (or during
    // development) re-executing the current binary is a more
    // deterministic way of starting a node.
    let ockam_exe = current_exe().unwrap_or_else(|_| "ockam".into());

    let (mlog, elog) = cfg.node_log_paths(name).unwrap();

    let main_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mlog)
        .context("failed to open log path")?;

    let stderr_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(elog)
        .context("failed to open stderr log path")?;

    let mut args = vec![
        match verbose {
            0 => "-vv".to_string(),
            v => format!("-{}", "v".repeat(v as usize)),
        },
        "--no-color".to_string(),
        "node".to_string(),
        "create".to_string(),
        "--tcp-listener-address".to_string(),
        address.to_string(),
        "--foreground".to_string(),
        "--child-process".to_string(),
    ];

    if let Some(path) = project {
        args.push("--project".to_string());
        let p = path
            .to_str()
            .unwrap_or_else(|| panic!("unsupported path {path:?}"));
        args.push(p.to_string())
    }

    if skip_defaults {
        args.push("--skip-defaults".to_string());
    }

    if no_shared_identity {
        args.push("--no-shared-identity".to_string());
    }

    if enable_credential_checks {
        args.push("--enable-credential-checks".to_string());
    }

    if let Some(c) = invite {
        args.push("--enrollment-token".to_string());
        args.push(hex::encode(c.code()))
    }

    args.push(name.to_owned());

    let child = Command::new(ockam_exe)
        .args(args)
        .stdout(main_log_file)
        .stderr(stderr_log_file)
        .spawn()?;

    // Update the pid in the config (should we remove this?)
    cfg.set_node_pid(name, child.id() as i32)?;
    cfg.persist_config_updates()?;

    Ok(())
}
