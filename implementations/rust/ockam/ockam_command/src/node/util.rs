use std::sync::Arc;

use anyhow::{anyhow, Context as _, Result};

use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::config::cli::OckamConfig as OckamConfigApi;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::{IdentityOverride, NodeManager, NODEMANAGER_ADDR};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::Vault;
use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};
use tracing::trace;

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::{project, OckamConfig};
use crate::{util::startup, CommandGlobalOpts};

pub async fn start_embedded_node(ctx: &Context, cfg: &OckamConfig) -> Result<String> {
    let cmd = CreateCommand::default();

    // Create node directory if it doesn't exist
    tokio::fs::create_dir_all(&cfg.get_node_dir_raw(&cmd.node_name)?).await?;

    // This node was initially created as a foreground node
    if !cmd.child_process {
        create_default_identity_if_needed(ctx, cfg).await?;
    }

    let identity_override = if cmd.skip_defaults {
        None
    } else {
        Some(get_identity_override(ctx, cfg).await?)
    };

    let project_id = match &cmd.project {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let p: ProjectInfo = serde_json::from_str(&s)?;
            let project_id = p.id.as_bytes().to_vec();
            project::config::set_project(cfg, &(&p).into()).await?;
            add_project_authority(p, &cmd.node_name, cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;
    let node_dir = cfg.get_node_dir_raw(&cmd.node_name)?;
    let node_man = NodeManager::create(
        NODEMANAGER_ADDR,
        ctx,
        cmd.node_name.clone(),
        node_dir,
        identity_override,
        cmd.skip_defaults || cmd.launch_config.is_some(),
        cmd.enable_credential_checks,
        Some(&cfg.authorities(&cmd.node_name)?.snapshot()),
        project_id,
        (TransportType::Tcp, TransportMode::Listen, bind),
        tcp,
    )
    .await?;

    ctx.start_worker(NODEMANAGER_ADDR, node_man).await?;

    Ok(cmd.node_name.clone())
}

pub(super) async fn create_default_identity_if_needed(
    ctx: &Context,
    cfg: &OckamConfig,
) -> Result<()> {
    // Get default root vault (create if needed)
    let default_vault_path = cfg.get_default_vault_path().unwrap_or_else(|| {
        let default_vault_path = cli::OckamConfig::directories()
            .config_dir()
            .join("default_vault.json");

        cfg.set_default_vault_path(Some(default_vault_path.clone()));

        default_vault_path
    });

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let vault = Vault::new(Some(Arc::new(storage)));

    // Get default root identity (create if needed)
    if cfg.get_default_identity().is_none() {
        let identity = Identity::create(ctx, &vault).await?;
        let exported_data = identity.export().await?;
        cfg.set_default_identity(Some(exported_data));
    };

    cfg.persist_config_updates()?;

    Ok(())
}

pub(super) async fn get_identity_override(
    ctx: &Context,
    cfg: &OckamConfig,
) -> Result<IdentityOverride> {
    // Get default root vault
    let default_vault_path = cfg
        .get_default_vault_path()
        .context("Default vault was not found")?;

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let vault = Vault::new(Some(Arc::new(storage)));

    // Get default root identity
    let default_identity = cfg
        .get_default_identity()
        .context("Default identity was not found")?;

    // Just to check validity
    Identity::import(ctx, &default_identity, &vault).await?;

    Ok(IdentityOverride {
        identity: default_identity,
        vault_path: default_vault_path,
    })
}

pub(super) async fn add_project_authority(
    p: ProjectInfo<'_>,
    node: &str,
    cfg: &OckamConfig,
) -> Result<()> {
    let m = p
        .authority_access_route
        .map(|a| MultiAddr::try_from(&*a))
        .transpose()?;
    let a = p
        .authority_identity
        .map(|a| hex::decode(a.as_bytes()))
        .transpose()?;
    if let Some((a, m)) = a.zip(m) {
        let v = Vault::default();
        let i = PublicIdentity::import(&a, &v).await?;
        let a = cli::Authority::new(a, m);
        cfg.authorities(node)?
            .add_authority(i.identifier().clone(), a)
    } else {
        Err(anyhow!("missing authority in project info"))
    }
}

pub async fn delete_embedded_node(cfg: &OckamConfig, name: &str) {
    // Try removing the node's directory
    if let Ok(dir) = cfg.get_node_dir_raw(name) {
        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}

pub fn delete_all_nodes(opts: CommandGlobalOpts, force: bool) -> anyhow::Result<()> {
    // Try to delete all nodes found in the config file + their associated processes
    let nn: Vec<String> = {
        let inner = &opts.config.inner();
        inner.nodes.iter().map(|(name, _)| name.clone()).collect()
    };
    for node_name in nn.iter() {
        delete_node(&opts, node_name, force)
    }

    // Try to delete dangling embedded nodes directories
    let dirs = OckamConfigApi::directories();
    let nodes_dir = dirs.data_local_dir();
    if nodes_dir.exists() {
        for entry in nodes_dir.read_dir()? {
            let dir = entry?;
            if !dir.file_type()?.is_dir() {
                continue;
            }
            if let Some(dir_name) = dir.file_name().to_str() {
                if !nn.contains(&dir_name.to_string()) {
                    let _ = std::fs::remove_dir_all(dir.path());
                }
            }
        }
    }

    // If force is enabled
    if force {
        // delete the config and nodes directories
        opts.config.remove()?;
        // and all dangling/orphan ockam processes
        if let Ok(cpid) = get_current_pid() {
            let s = System::new_all();
            for (pid, process) in s.processes() {
                if pid != &cpid && process.name() == "ockam" {
                    process.kill();
                }
            }
        }
    } else if let Err(e) = opts.config.persist_config_updates() {
        eprintln!("Failed to update config file. You might need to run the command with --force to delete all config directories");
        return Err(e);
    }
    Ok(())
}

pub fn delete_node(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) {
    trace!(%node_name, "Deleting node");

    // We ignore the result of killing the node process as it could be not
    // found (after a restart or if the user manually deleted it, for example).
    let _ = delete_node_pid(opts, node_name, sigkill);

    delete_node_config(opts, node_name);
}

fn delete_node_pid(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) -> anyhow::Result<()> {
    trace!(%node_name, "Deleting node pid");
    // Stop the process PID if it has one assigned in the config file
    if let Some(pid) = opts.config.get_node_pid(node_name)? {
        startup::stop(pid, sigkill)?;
        // Give some room for the process to stop
        std::thread::sleep(std::time::Duration::from_millis(100));
        // If it fails to bind, the port is still in use, so we try again to stop the process
        let addr = format!("127.0.0.1:{}", opts.config.get_node_port(node_name));
        if std::net::TcpListener::bind(&addr).is_err() {
            startup::stop(pid, sigkill)?;
        }
    }
    Ok(())
}

fn delete_node_config(opts: &CommandGlobalOpts, node_name: &str) {
    trace!(%node_name, "Deleting node config");

    // Try removing the node's directory.
    // If the directory is not found, we ignore the result and continue.
    let _ = opts
        .config
        .get_node_dir_raw(node_name)
        .map(std::fs::remove_dir_all);

    // Try removing the node's info from the config file.
    opts.config.remove_node(node_name);
}

pub mod run {
    use std::collections::VecDeque;
    use std::env::current_exe;
    use std::path::{Path, PathBuf};
    use std::process::Stdio;

    use tracing::trace;

    use ockam_core::compat::collections::HashMap;

    use super::*;

    pub struct CommandsRunner {
        path: PathBuf,
        commands: HashMap<String, Command>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    #[cfg_attr(test, derive(Clone, Debug, PartialEq))]
    struct Command {
        args: Vec<String>,
        depends_on: Option<String>,
    }

    impl CommandsRunner {
        fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
            let path = path.as_ref();
            let commands = if path.exists() {
                let s = std::fs::read_to_string(path)?;
                serde_json::from_str(&s)?
            } else {
                HashMap::new()
            };
            Ok(Self {
                path: path.into(),
                commands,
            })
        }

        pub fn export<P: AsRef<Path>>(
            path: P,
            export_as: String,
            depends_on: Option<String>,
            args: Vec<String>,
        ) -> Result<()> {
            let path = path.as_ref();
            let mut c = Self::new(path)?;
            c.add(export_as, depends_on, args);
            c.update()?;
            Ok(())
        }

        fn add(&mut self, name: String, depends_on: Option<String>, args: Vec<String>) {
            let args = Self::cleanup_args(args);
            self.commands.insert(name, Command { args, depends_on });
        }

        fn cleanup_args(mut args: Vec<String>) -> Vec<String> {
            let mut cleaned = vec![];
            // Remove the command executable path
            args.remove(0);
            // Remove export arguments
            let export_args = ["--export", "--export-as", "--depends-on"];
            let mut it = args.into_iter();
            while let Some(arg) = it.next() {
                if export_args.contains(&&*arg) {
                    // Skip argument value
                    it.next();
                } else {
                    cleaned.push(arg);
                }
            }
            cleaned
        }

        fn update(self) -> Result<()> {
            let s = serde_json::to_string_pretty(&self.commands)
                .context("Failed to convert commands to json format")?;
            std::fs::write(&self.path, &s).context("Failed to write commands to file")?;
            Ok(())
        }

        /// Run all commands sorted based on their dependencies
        pub fn run<P: AsRef<Path>>(path: P) -> Result<()> {
            let c = Self::new(path)?;
            let ockam = current_exe().unwrap_or_else(|_| "ockam".into());
            for cmd in c.sort_commands()? {
                std::thread::sleep(std::time::Duration::from_millis(250));
                trace!("Running command with args {:?}", cmd.args);
                println!("Running command with args '{}'", cmd.args.join(" "));
                std::process::Command::new(&ockam)
                    .args(cmd.args)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()?;
            }
            Ok(())
        }

        /// Sort list of commands based on their dependencies
        fn sort_commands(self) -> Result<Vec<Command>> {
            // Check that `depend_on` reference to existing commands
            for (name, cmd) in &self.commands {
                if let Some(depends_on) = &cmd.depends_on {
                    if !self.commands.contains_key(depends_on) {
                        return Err(anyhow!(
                            "Command '{}' depends on non-existing command '{}'",
                            name,
                            depends_on
                        ));
                    }
                }
            }
            // Convert hashmap to a ring buffer so we can cycle through the commands multiple times.
            let mut commands: VecDeque<(String, Command)> =
                self.commands.into_iter().map(|(k, v)| (k, v)).collect();
            // We will store how many times we cycle through each command.
            let mut cycles: HashMap<String, usize> = HashMap::new();
            let max_cycles = commands.len();
            // We will store the commands with unresolved dependencies (using `name` and `depends_on`).
            let mut unresolved = HashMap::new();
            // Other variables to store the sorting state.
            let mut names = vec![];
            let mut sorted = vec![];
            // Process commands until we have them all sorted and there are none left on the list.
            while let Some((name, cmd)) = commands.pop_front() {
                // Process command dependencies.
                if let Some(depends_on) = &cmd.depends_on {
                    // Dependency is in a previous position of the list. We can push the current command to the sorted list.
                    if names.contains(depends_on) {
                        names.push(name);
                        sorted.push(cmd);
                    }
                    // Dependency is in a further position of the list.
                    else {
                        // In the worst-case scenario, we will sort one element on each cycle, so we will sort the complete list
                        // in `max_cycles` cycles. Otherwise, we have a circular dependency.
                        if let Some(c) = cycles.get_mut(&name) {
                            let curr = *c;
                            *c = curr + 1;
                            if *c > max_cycles {
                                let msg = unresolved
                                    .into_iter()
                                    .map(|(k, v)| format!("command {k} -> depends_on {v}"))
                                    .collect::<Vec<_>>();
                                return Err(anyhow!("Circular dependency detected for {msg:?}"));
                            }
                        } else {
                            // Initialize cycle counter for the current command.
                            cycles.insert(name.clone(), 1);
                        }

                        // If the `depends_on` is on the unresolved hashmap and the command name matches the item's value, then
                        // we have a circular dependency. Example:
                        // 1. name=A, depends_on=B -> we store (A,B) in the dependencies hashmap
                        // 2. name=B, depends_on=A -> we find "A" key with "B" value -> circular dependency
                        if let Some(n) = unresolved.get(depends_on) {
                            if name.eq(n) {
                                return Err(anyhow!(
                                        "Circular dependency detected for commands {name} <-> {depends_on}",
                                    ));
                            }
                        }
                        // We requeue the command at the end of the list and we also store it in the unresolved hashmap.
                        unresolved.insert(name.clone(), depends_on.clone());
                        commands.push_back((name, cmd));
                    }
                }
                // Command has no dependencies. We can add it right away to the sorted list.
                else {
                    sorted.push(cmd);
                    names.push(name);
                }
            }
            Ok(sorted)
        }
    }

    #[cfg(test)]
    mod tests {
        use tempfile::tempdir;

        use super::*;

        #[test]
        fn create_from_empty_file() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.path, file_path);
            assert!(cr.commands.is_empty());
        }

        #[test]
        fn create_from_existing_file() {
            let contents = r#"{
                "command1": {
                    "args": ["arg1", "arg2"],
                    "depends_on": null
                },
                "command2": {
                    "args": ["arg3", "arg4"],
                    "depends_on": "command1"
                }
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.path, file_path);
            assert_eq!(cr.commands.len(), 2);

            let cmd1 = cr.commands.get("command1").expect("Failed to get command1");
            assert_eq!(cmd1.args, vec!["arg1", "arg2"]);
            assert!(cmd1.depends_on.is_none());

            let cmd2 = cr.commands.get("command2").expect("Failed to get command2");
            assert_eq!(cmd2.args, vec!["arg3", "arg4"]);
            assert_eq!(cmd2.depends_on, Some("command1".to_string()));
        }

        #[test]
        fn cleanup_args() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--export-as".to_string(),
                "ndblue".to_string(),
                "--depends-on".to_string(),
                "enroll".to_string(),
            ];
            let cleaned = CommandsRunner::cleanup_args(args);
            assert_eq!(cleaned, vec!["node", "create", "blue"]);
        }

        #[test]
        fn add() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let mut cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");

            let export_as = "ndblue".to_string();
            let depends_on = "enroll".to_string();
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--export-as".to_string(),
                export_as.clone(),
                "--depends-on".to_string(),
                depends_on.clone(),
            ];
            let cleaned_args = vec!["node", "create", "blue"];

            cr.add(export_as.clone(), Some(depends_on.clone()), args);
            assert_eq!(cr.commands.len(), 1);
            let cmd = cr.commands.get(&export_as).expect("Failed to get command");
            assert_eq!(cmd.args, cleaned_args);
            assert_eq!(cmd.depends_on, Some(depends_on));
        }

        #[test]
        fn export() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let export_as = "ndblue".to_string();
            let depends_on = None;
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--export-as".to_string(),
                export_as.clone(),
            ];
            let cleaned_args = vec!["node", "create", "blue"];
            CommandsRunner::export(file_path.clone(), export_as.clone(), depends_on, args)
                .expect("Failed to export");

            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.commands.len(), 1);
            let cmd = cr
                .commands
                .get(&export_as)
                .expect("Failed to retrieve command from file");
            assert_eq!(cmd.args, cleaned_args);
            assert!(cmd.depends_on.is_none());
        }

        #[test]
        fn sort_commands() {
            let contents = r#"{
                "c": {
                    "args": ["c", "arg"],
                    "depends_on": "a"
                },
                "b": {
                    "args": ["b", "arg"],
                    "depends_on": "c"
                },
                "a": {
                    "args": ["a", "arg"],
                    "depends_on": null
                }
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            let res = cr.sort_commands().expect("Failed to sort commands");
            let expected = vec![
                Command {
                    args: vec!["a".to_string(), "arg".to_string()],
                    depends_on: None,
                },
                Command {
                    args: vec!["c".to_string(), "arg".to_string()],
                    depends_on: Some("a".to_string()),
                },
                Command {
                    args: vec!["b".to_string(), "arg".to_string()],
                    depends_on: Some("c".to_string()),
                },
            ];
            assert_eq!(res, expected);
        }

        #[test]
        fn sort_depends_on_points_to_invalid_command() {
            let contents = r#"{
                "b": {
                    "args": ["b", "arg"],
                    "depends_on": "c"
                },
                "a": {
                    "args": ["a", "arg"],
                    "depends_on": null
                }
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            let res = cr.sort_commands();
            assert!(res.is_err());
        }

        #[test]
        fn sort_commands_circular_dependency_direct() {
            let contents = r#"{
                "a": {
                    "args": ["a", "arg"],
                    "depends_on": "b"
                },
                "b": {
                    "args": ["b", "arg"],
                    "depends_on": "a"
                }
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            let res = cr.sort_commands();
            assert!(res.is_err());
        }

        #[test]
        fn sort_commands_circular_dependency_indirect() {
            let contents = r#"{
                "a": {
                    "args": ["a", "arg"],
                    "depends_on": "d"
                },
                "b": {
                    "args": ["b", "arg"],
                    "depends_on": "a"
                },
                "c": {
                    "args": ["c", "arg"],
                    "depends_on": null
                },
                "d": {
                    "args": ["d", "arg"],
                    "depends_on": "b"
                }
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            let res = cr.sort_commands();
            assert!(res.is_err());
        }
    }
}
