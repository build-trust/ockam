use std::sync::Arc;

use anyhow::{anyhow, Context as _, Result};
use ockam::vault::{Secret, SecretPersistence, SecretType};
use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};
use tracing::trace;

use ockam::identity::{Identity, PublicIdentity};
use ockam::{Context, TcpTransport};
use ockam_api::config::cli;
use ockam_api::config::cli::OckamConfig as OckamConfigApi;
use ockam_api::nodes::models::transport::{TransportMode, TransportType};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
};
use ockam_api::nodes::{IdentityOverride, NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_multiaddr::MultiAddr;
use ockam_vault::storage::FileStorage;
use ockam_vault::{KeyId, SecretAttributes, SecretVault, Vault};

use crate::node::CreateCommand;
use crate::project::ProjectInfo;
use crate::{project, OckamConfig};
use crate::{util::startup, CommandGlobalOpts};

pub async fn start_embedded_node(ctx: &Context, cfg: &OckamConfig) -> Result<String> {
    let cmd = CreateCommand::default();

    // Create node directory if it doesn't exist
    tokio::fs::create_dir_all(&cfg.get_node_dir_unchecked(&cmd.node_name)).await?;

    // This node was initially created as a foreground node
    if !cmd.child_process {
        create_default_identity_if_needed(ctx, cfg, None).await?;
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
            let project_id = p.id.to_string();
            project::config::set_project(cfg, &(&p).into()).await?;
            add_project_authority(p, &cmd.node_name, cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(ctx).await?;
    let bind = cmd.tcp_listener_address;
    tcp.listen(&bind).await?;
    let node_dir = cfg.get_node_dir_unchecked(&cmd.node_name);
    let projects = cfg.inner().lookup().projects().collect();
    let node_man = NodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(
            cmd.node_name.clone(),
            node_dir,
            cmd.skip_defaults || cmd.launch_config.is_some(),
            identity_override,
            cfg.is_aws_kms_enabled(),
        ),
        NodeManagerProjectsOptions::new(
            Some(&cfg.authorities(&cmd.node_name)?.snapshot()),
            project_id,
            projects,
            None,
        ),
        NodeManagerTransportOptions::new((TransportType::Tcp, TransportMode::Listen, bind), tcp),
    )
    .await?;

    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await?;

    Ok(cmd.node_name.clone())
}

pub(super) async fn create_default_identity_if_needed(
    ctx: &Context,
    cfg: &OckamConfig,
    kid: Option<&KeyId>,
) -> Result<()> {
    // Get default root vault (create if needed)
    let default_vault_path = cfg.get_default_vault_path().unwrap_or_else(|| {
        let default_vault_path = cli::OckamConfig::dir().join("default_vault.json");
        cfg.set_default_vault_path(Some(default_vault_path.clone()));
        default_vault_path
    });

    let storage = FileStorage::create(default_vault_path.clone()).await?;
    let mut vault = Vault::new(Some(Arc::new(storage)));

    if cfg.is_aws_kms_enabled() {
        vault.enable_aws_kms().await?
    }

    // Get default root identity (create if needed)
    if cfg.get_default_identity().is_none() {
        let identity = if let Some(kid) = kid {
            let attrs =
                SecretAttributes::new(SecretType::NistP256, SecretPersistence::Persistent, 32);
            let kid = vault
                .secret_import(Secret::Ref(kid.to_string()), attrs)
                .await?;
            Identity::create_with_key(ctx, &vault, &kid).await?
        } else {
            Identity::create(ctx, &vault).await?
        };
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
    let mut vault = Vault::new(Some(Arc::new(storage)));

    if cfg.is_aws_kms_enabled() {
        vault.enable_aws_kms().await?
    }

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
    let dir = cfg.get_node_dir_unchecked(name);
    let _ = tokio::fs::remove_dir_all(dir).await;
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
    let nodes_dir = OckamConfigApi::nodes_dir();
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
        let addr = format!(
            "127.0.0.1:{}",
            opts.config.get_node_port(node_name).unwrap()
        );
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
    let dir = opts.config.get_node_dir_unchecked(node_name);
    let _ = std::fs::remove_dir_all(dir);

    // Try removing the node's info from the config file.
    opts.config.remove_node(node_name);
}

pub mod run {
    use std::env::current_exe;
    #[cfg(test)]
    use std::fmt::{Display, Formatter};
    use std::io::Write;
    use std::iter::Peekable;
    use std::path::{Path, PathBuf};
    use std::process::Stdio;
    use std::slice::Iter;
    use std::str::FromStr;

    use clap::Parser;
    use ockam_api::nodes::config::{Command, Commands};
    use tracing::trace;

    use ockam_multiaddr::proto::Node;

    use crate::OckamCommand;

    use super::*;

    pub struct CommandsRunner {
        exe: PathBuf,
        path: PathBuf,
        pub commands: Commands,
    }

    #[derive(clap::ValueEnum, Clone, Debug)]
    pub enum CommandSection {
        OnNodeInit,
        OnNodeStartup,
    }

    impl Default for CommandSection {
        fn default() -> Self {
            Self::OnNodeInit
        }
    }

    #[cfg(test)]
    impl Display for CommandSection {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                CommandSection::OnNodeInit => "node-init",
                CommandSection::OnNodeStartup => "node-startup",
            };
            write!(f, "{}", s)
        }
    }

    impl CommandsRunner {
        pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
            let path = path.as_ref();
            let commands = if path.exists() {
                let s = std::fs::read_to_string(path)?;
                serde_json::from_str(&s)?
            } else {
                Commands::default()
            };
            Ok(Self {
                exe: current_exe().unwrap_or_else(|_| "ockam".into()),
                path: path.into(),
                commands,
            })
        }

        pub fn export<P: AsRef<Path>>(
            path: P,
            section: CommandSection,
            args: Vec<String>,
            pipe: Option<bool>,
        ) -> Result<()> {
            let path = path.as_ref();
            let mut cr = Self::new(path)?;
            cr.add(section, args, pipe);
            cr.persist()?;
            Ok(())
        }

        fn add(&mut self, section: CommandSection, args: Vec<String>, pipe: Option<bool>) {
            let command = Command::new(Self::cleanup_export_args(args).join(" "), pipe);
            match section {
                CommandSection::OnNodeInit => self.commands.on_node_init.push(command),
                CommandSection::OnNodeStartup => self.commands.on_node_startup.push(command),
            };
        }

        /// Remove all "export" related arguments.
        fn cleanup_export_args(mut args: Vec<String>) -> Vec<String> {
            let mut cleaned = vec![];
            // Remove the command executable path
            args.remove(0);
            // Remove export arguments
            let export_args = ["--export", "--export-section", "--pipe"];
            let mut it = args.into_iter();
            while let Some(arg) = it.next() {
                if export_args.contains(&&*arg) {
                    // Continue to next argument if it's a `flag` argument
                    if ["--pipe"].contains(&&*arg) {
                        continue;
                    }
                    // Skip next item (argument value) if it's a `key=value` argument
                    else {
                        it.next();
                    }
                } else {
                    cleaned.push(arg);
                }
            }
            cleaned
        }

        /// Persist commands to file.
        fn persist(self) -> Result<()> {
            let s = serde_json::to_string_pretty(&self.commands)
                .context("Failed to convert commands to json format")?;
            std::fs::write(&self.path, &s).context("Failed to write commands to file")?;
            Ok(())
        }

        /// Create a node given the arguments from the "run" section
        pub fn run<P: AsRef<Path>>(path: P) -> Result<()> {
            let cr = Self::new(path)?;
            match cr.commands.run {
                None => Err(anyhow!("Couldn't create node: `run` section not defined")),
                Some(r) => {
                    let args = r.args(&cr.path, cr.exe.to_str().expect("Invalid executable path"));
                    let cmd: OckamCommand = OckamCommand::parse_from(args);
                    cmd.run();
                    Ok(())
                }
            }
        }

        /// Run "on_node_init" commands section
        pub fn run_node_init<P: AsRef<Path>>(path: P) -> Result<()> {
            let cr = Self::new(path)?;
            let cmds = cr.commands.on_node_init;
            let mut it = cmds.iter().peekable();
            // Node was just created, prompt user before executing the first command
            CommandsRunner::wait_for_prompt(it.peek())?;
            CommandsRunner::go(&cr.exe, it)
        }

        /// Run "on_node_startup" commands section
        pub fn run_node_startup<P: AsRef<Path>>(path: P) -> Result<()> {
            let cr = Self::new(path)?;
            let cmds = cr.commands.on_node_startup;
            let it = cmds.iter().peekable();
            CommandsRunner::go(&cr.exe, it)
        }

        /// Execute the list of commands
        fn go(exe: &PathBuf, mut it: Peekable<Iter<Command>>) -> Result<()> {
            let mut prev_output: Option<Vec<u8>> = None;
            let mut stdin = Stdio::null();
            while let Some(cmd) = it.next() {
                CommandsRunner::command_preprocessing(exe, cmd)?;
                let args = cmd.args();
                trace!("Running command `{:?}`", &args);
                println!("\nRunning command '{}'", &args.join(" "));

                // We have different scenarios based on the `pipe` field
                //  1. Pipe output to next command
                //  2. Pipe input from previous command
                //  3. Both 1 and 2
                let mut child = std::process::Command::new(exe)
                    .args(&args)
                    .stdout(Stdio::piped())
                    .stdin(stdin)
                    .stderr(Stdio::inherit())
                    .spawn()?;

                // Write previous output to stdin
                if let Some(input) = prev_output.take() {
                    let mut stdin = child.stdin.take().expect("Failed to open stdin");
                    std::thread::spawn(move || {
                        stdin.write_all(&input).expect("Failed to write to stdin");
                    });
                }

                let output = child.wait_with_output()?;

                // Stdout is piped to the child process, so we print it here in the current process.
                print!("\n{}", String::from_utf8_lossy(&output.stdout));

                // Stop processing any further commands if the current command failed
                if !output.status.success() {
                    return Err(anyhow!("Command returned non-zero exit code"));
                }

                // Save output for next command
                if cmd.pipe_output() {
                    stdin = Stdio::piped();
                    prev_output = Some(output.stdout);
                } else {
                    stdin = Stdio::null();
                }

                // If command was `node create`, then prompt the user before continuing to the next command.
                let args = cmd.args();
                if args.len() >= 2 && &args[0] == "node" && &args[1] == "create" {
                    CommandsRunner::wait_for_prompt(it.peek())?;
                }

                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Ok(())
        }

        /// Parse current command to find the necessary steps to be run before it.
        ///
        /// E.g. a command that has a `/node/blue` argument will trigger the creation of a node named `blue`.
        fn command_preprocessing(exe: &PathBuf, cmd: &Command) -> Result<()> {
            // The config file can be updated by other commands that are run on different threads,
            // so we load the config file each time we run a command so we always have an updated version.
            let lookup = OckamConfig::load()?;
            for arg in &cmd.args() {
                // Parse arguments to find a `/node/<name>/...` instance and create a node with that name if it doesn't exist.
                if arg.starts_with("/node/") {
                    let node_name = {
                        let maddr = MultiAddr::from_str(arg)?;
                        let mut it = maddr.iter().peekable();
                        let p = it.next().context("Should have a first value")?;
                        let name = p
                            .cast::<Node>()
                            .context("Failed to parse node name from address")?;
                        if lookup.get_node(&name).is_ok() {
                            // Node already exists, continue to next iteration.
                            continue;
                        }
                        name.to_string()
                    };
                    println!("Creating node '{node_name}'");
                    let mut args = vec!["node".into(), "create".into(), node_name];
                    let mut optional_args = CommandsRunner::ask_for_node_args()?;
                    args.append(&mut optional_args);
                    trace!("Running command {:?}", args);
                    let status = std::process::Command::new(exe)
                        .args(args)
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .status()?;
                    if !status.success() {
                        return Err(anyhow!("Failed to create node"));
                    }
                    CommandsRunner::wait_for_prompt(Some(&cmd))?;
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
            }
            Ok(())
        }

        /// Waits for the user to press `Enter` before proceeding with the next command.
        /// If there is no next command, the prompt will be skipped.
        fn wait_for_prompt(next_cmd: Option<&&Command>) -> Result<()> {
            if let Some(next_cmd) = next_cmd {
                print!(
                    "Press `Enter` to continue to the next command: `{}`",
                    next_cmd.args().join(" ")
                );
                let mut input = String::new();
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut input)?;
            }
            Ok(())
        }

        /// Prompt the user for the optional arguments to be passed to the `node create` command.
        fn ask_for_node_args() -> Result<Vec<String>> {
            let mut args = vec![];
            println!("Enter optional arguments for the node (e.g. `--project project.json`):");
            let mut input = String::new();
            std::io::stdout().flush()?;
            std::io::stdin().read_line(&mut input)?;
            if input.trim().is_empty() {
                println!("No optional arguments provided");
            } else {
                args.push(input.trim().to_string());
                println!("The optional arguments are: `{}`", args.join(" "));
            }
            print!("Proceed? [Y/n] ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" || input.trim().is_empty() {
                // Convert to single-value entries. E.g. ["--flag", "--arg value"] -> ["--flag", "--arg", "value"]
                Ok(args
                    .join(" ")
                    .split(' ')
                    .map(|s| s.to_string())
                    .filter(|x| !x.is_empty())
                    .collect())
            } else {
                CommandsRunner::ask_for_node_args()
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use ockam_api::nodes::config::CommandArgs;
        use tempfile::tempdir;

        use super::*;

        #[test]
        fn create_from_empty_file() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.path, file_path);
            assert!(cr.commands.on_node_init.is_empty());
            assert!(cr.commands.on_node_startup.is_empty());
        }

        #[test]
        fn create_from_existing_file() {
            let contents = r#"{
                "run": {
                    "name": "my_node",
                    "args": "--project project.json"
                },
                "on_node_init": [
                    {
                        "args": ["init", "1"],
                        "pipe": true
                    },
                    {
                        "command": "init",
                        "args": ["2"]
                    }
                ],
                "on_node_startup": [
                    "startup 3",
                    {
                        "command": "startup 4"
                    }
                ]
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.path, file_path);

            // Run section
            let run = cr.commands.run.expect("Failed to parse run section");
            assert_eq!(run.name, "my_node");
            assert_eq!(
                run.args.unwrap(),
                CommandArgs::String("--project project.json".into())
            );

            // Node init section
            assert_eq!(cr.commands.on_node_init.len(), 2);

            let cmd1 = cr.commands.on_node_init.get(0).expect("Failed to get cmd1");
            assert_eq!(cmd1.args(), vec!["init", "1", "--pipe"]);
            assert!(cmd1.pipe_output());

            let cmd2 = cr.commands.on_node_init.get(1).expect("Failed to get cmd2");
            assert_eq!(cmd2.args(), vec!["init", "2"]);
            assert!(!cmd2.pipe_output());

            // Node startup section
            assert_eq!(cr.commands.on_node_startup.len(), 2);

            let cmd3 = cr
                .commands
                .on_node_startup
                .get(0)
                .expect("Failed to get cm3");
            assert_eq!(cmd3.args(), vec!["startup", "3"]);
            assert!(!cmd3.pipe_output());

            let cmd4 = cr
                .commands
                .on_node_startup
                .get(1)
                .expect("Failed to get cmd4");
            assert_eq!(cmd4.args(), vec!["startup", "4"]);
            assert!(!cmd4.pipe_output());
        }

        #[test]
        fn create_from_existing_file_with_single_section() {
            let contents = r#"{
                "on_node_init": [
                    "init 1",
                    "init 2"
                ]
            }"#;
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            std::fs::write(&file_path, contents).expect("Failed to write contents to file");
            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.path, file_path);

            // Node init section
            assert_eq!(cr.commands.on_node_init.len(), 2);

            let cmd1 = cr.commands.on_node_init.get(0).expect("Failed to get cmd1");
            assert_eq!(cmd1.args(), vec!["init", "1"]);
            assert!(!cmd1.pipe_output());

            let cmd2 = cr.commands.on_node_init.get(1).expect("Failed to get cmd2");
            assert_eq!(cmd2.args(), vec!["init", "2"]);
            assert!(!cmd2.pipe_output());
        }

        #[test]
        fn parse_command_args() {
            let expected_args = vec!["startup", "--arg", "value"];
            let assert = |contents: &str| {
                let cmd: Command = serde_json::from_str(contents).expect("Failed to parse command");
                assert_eq!(&cmd.args(), &expected_args);
            };

            // Plain string
            let contents = r#""startup --arg value""#;
            assert(contents);

            // Args list with single-entry values
            let contents = r#"{
                "args": ["startup", "--arg", "value"]
            }"#;
            assert(contents);

            // Args list with multi-entry values
            let contents = r#"{
                "args": ["startup", "--arg value"]
            }"#;
            assert(contents);

            // One-liner args
            let contents = r#"{
                "args": "startup --arg value"
            }"#;
            assert(contents);

            // With command field + args list
            let contents = r#"{
                "command": "startup",
                "args": "--arg value"
            }"#;
            assert(contents);

            // One-liner command
            let contents = r#"{
                "command": "startup --arg value"
            }"#;
            assert(contents);
        }

        #[test]
        fn cleanup_export_args() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--pipe".to_string(),
            ];
            let cleaned = CommandsRunner::cleanup_export_args(args);
            assert_eq!(cleaned, vec!["node", "create", "blue"]);
        }

        #[test]
        fn add() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let mut cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");

            let section = CommandSection::OnNodeStartup;
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--export-section".to_string(),
                section.to_string(),
            ];
            let cleaned_args = vec!["node", "create", "blue"];

            cr.add(CommandSection::OnNodeStartup, args, None);
            assert_eq!(cr.commands.on_node_startup.len(), 1);
            let cmd = cr
                .commands
                .on_node_startup
                .get(0)
                .expect("Failed to get command");
            assert_eq!(cmd.args(), cleaned_args);
            assert!(!cmd.pipe_output());
        }

        #[test]
        fn export() {
            let dir = tempdir().expect("Failed to create temp dir");
            let file_path = dir.path().join("cmds.json");
            let section = CommandSection::OnNodeInit;
            let pipe = Some(true);
            let args = vec![
                "ockam".to_string(),
                "node".to_string(),
                "create".to_string(),
                "blue".to_string(),
                "--export".to_string(),
                file_path.to_str().unwrap().to_string(),
                "--export-section".to_string(),
                section.to_string(),
                "--pipe".to_string(),
            ];
            CommandsRunner::export(file_path.clone(), section, args, pipe)
                .expect("Failed to export");

            let cr = CommandsRunner::new(&file_path).expect("Failed to create CommandsRunner");
            assert_eq!(cr.commands.on_node_init.len(), 1);
            let cmd = cr
                .commands
                .on_node_init
                .get(0)
                .expect("Failed to retrieve command from file");
            assert_eq!(cmd.args(), vec!["node", "create", "blue", "--pipe"]);
            assert!(cmd.pipe_output());
        }
    }
}
