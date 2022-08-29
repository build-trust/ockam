use crate::{help, node::HELP_DETAIL, util::startup, CommandGlobalOpts};
use clap::Args;
use sysinfo::{get_current_pid, ProcessExt, System, SystemExt};
use tracing::{debug, trace};

/// Delete Nodes
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct DeleteCommand {
    /// Name of the node.
    #[clap(default_value = "default", hide_default_value = true, group = "nodes")]
    node_name: String,

    /// Terminate all nodes
    #[clap(long, short, group = "nodes")]
    all: bool,

    /// Clean up config directories and all nodes state directories
    #[clap(display_order = 901, long, short)]
    force: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{}", e);
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: DeleteCommand) -> crate::Result<()> {
    if cmd.all {
        // Try to delete all nodes found in the config file
        let nn: Vec<String> = {
            let inner = &opts.config.get_inner();
            inner.nodes.iter().map(|(name, _)| name.clone()).collect()
        };
        for node_name in nn.iter() {
            // Deleting a node can fail if there are inconsistencies between the config file and the
            // node state directory (e.g. due to manual manipulation of the config file).
            // We fail silently and continue with the next node.
            match delete_node(&opts, node_name, cmd.force) {
                Ok(_) => {
                    println!("Deleted node '{}'", node_name);
                }
                Err(e) => {
                    debug!(%node_name, ?e, "Failed to delete node");
                }
            }
        }
        // If force is enabled
        if cmd.force {
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
        }
        // If not, persist updates to the config file
        else if let Err(e) = opts.config.atomic_update().run() {
            eprintln!("Failed to update config file. You might need to run the command with --force to delete all config directories");
            return Err(crate::Error::new(crate::exitcode::IOERR, e));
        }
    } else {
        delete_node(&opts, &cmd.node_name, cmd.force)?;
        opts.config.atomic_update().run()?;
        println!("Deleted node '{}'", &cmd.node_name);
    }
    Ok(())
}

fn delete_node(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) -> anyhow::Result<()> {
    trace!(%node_name, "Deleting node");
    // Execute deletion operations without propagating errors
    let ops = [
        delete_node_pid(opts, node_name, sigkill),
        delete_node_config(opts, node_name),
    ];
    // Propagate errors once all operations have been executed
    for r in ops.into_iter() {
        r?;
    }
    Ok(())
}

fn delete_node_pid(opts: &CommandGlobalOpts, node_name: &str, sigkill: bool) -> anyhow::Result<()> {
    trace!(%node_name, "Deleting node pid");
    // Stop the process PID if it has one assigned in the config file
    if let Some(pid) = opts.config.get_node_pid(node_name)? {
        startup::stop(pid, sigkill)?;
        let addr = format!("127.0.0.1:{}", opts.config.get_node_port(node_name));
        // Give some room for the process to stop
        std::thread::sleep(std::time::Duration::from_millis(100));
        // If fails to bind, the port is already taken, so we try to stop the process
        if std::net::TcpListener::bind(&addr).is_err() {
            startup::stop(pid, sigkill)?;
        }
    }
    Ok(())
}

fn delete_node_config(opts: &CommandGlobalOpts, node_name: &str) -> anyhow::Result<()> {
    trace!(%node_name, "Deleting node config");
    // Try removing the node's directory
    let _ = opts
        .config
        .get_node_dir(node_name)
        .map(std::fs::remove_dir_all);
    // Remove the node's info from the config file
    opts.config.delete_node(node_name)?;
    Ok(())
}
