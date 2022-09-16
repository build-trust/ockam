//! An automatic setup mechanism for composition snippets

#![allow(unused)]

use crate::exitcode;
use crate::util::{ComposableSnippet, OckamConfig, Operation, RemoteMode, StartupConfig};
use anyhow::Context;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
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

/// Execute a series of commands to setup a node
pub fn start(node: &str, ockam_cfg: &OckamConfig, cfg: &StartupConfig) {
    let ockam = current_exe().unwrap_or_else(|_| "ockam".into());

    for ref snippet in cfg.get_all() {
        print!("Running: {} ...", snippet.op);
        run_snippet(&ockam, ockam_cfg, node, snippet);
    }
}

/// A utility function to spawn a new node into foreground mode
///
/// This function is used by `ockam node create` as well as `ockam
/// node start`, which attempts to re-use an existing node config
#[allow(clippy::too_many_arguments)]
pub fn spawn_node(
    ockam: &PathBuf,
    cfg: &OckamConfig,
    verbose: u8,
    skip_defaults: bool,
    enable_credential_checks: bool,
    name: &str,
    address: &str,
    project: Option<&Path>,
) {
    let (mlog, elog) = cfg.node_log_paths(name).unwrap();

    let main_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mlog)
        .expect("failed to open log path");

    let stderr_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(elog)
        .expect("failed to open stderr log path");

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

    if enable_credential_checks {
        args.push("--enable-credential-checks".to_string());
    }

    args.push(name.to_owned());

    let child = Command::new(ockam)
        .args(args)
        .stdout(main_log_file)
        .stderr(stderr_log_file)
        .spawn()
        .expect("could not spawn node");

    // Update the pid in the config (should we remove this?)
    cfg.set_node_pid(name, child.id() as i32)
        .expect("should never panic");

    // Save the config update
    if let Err(e) = cfg.persist_config_updates() {
        eprintln!("failed to update configuration: {}", e);
        std::process::exit(exitcode::IOERR);
    }
}

fn run_snippet(
    ockam: &PathBuf,
    cfg: &OckamConfig,
    node_name: &str,
    snippet @ ComposableSnippet { id, op, params }: &ComposableSnippet,
) {
    let args = match op {
        Operation::Node {
            api_addr,
            node_name: _,
        } => {
            // Starting the node is a special operation because it
            // doesn't directly map to any exposed operation (or
            // rather, ockam node start _is_ the exposed operation,
            // but it's also what is calling this code).  So, we
            // re-use the same launch mechanism as ockam node create
            // via a utility function.

            let verbose = cfg
                .get_node(node_name)
                .expect("failed to load node config")
                .verbose;

            spawn_node(
                ockam,     // The ockam CLI path
                cfg,       // Ockam configuration
                verbose,   // Previously user-chosen verbosity level
                true,      // skip-defaults because the node already exists
                false,     // Default value. TODO: implement persistence of this option
                node_name, // The selected node name
                api_addr,  // The selected node api address
                None,      // No project information available
            );

            // FIXME: Wait for the node to be ready
            std::thread::sleep(std::time::Duration::from_millis(500));

            println!("ok!");
            return;
        }
        Operation::Transport {
            mode,
            address,
            protocol: _,
        } => vec![
            "transport",
            "-vv",
            "create",
            "--reuse",
            "--node",
            node_name,
            match mode {
                RemoteMode::Connector => "tcp-connector",
                RemoteMode::Listener => "tcp-listener",
                RemoteMode::Receiver => unimplemented!(),
            },
            address,
        ],
        Operation::Portal {
            mode,
            protocol,
            bind,
            peer,
        } => {
            todo!()
        }
        Operation::SecureChannel => {
            todo!()
        }

        Operation::Forwarder => {
            todo!()
        }
    };

    Command::new(ockam)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();

    println!("ok");
}
