use crate::CommandGlobalOpts;
use clap::Args;
use colorful::Colorful;
use ockam_api::{fmt_log, fmt_warn};
use std::io;
use std::io::Read;
use tracing::{debug, info};

#[derive(Clone, Debug, Args, Default)]
pub struct ForegroundArgs {
    /// Run the node in foreground mode. This will block the current process until the node receives
    /// an exit signal (e.g., SIGINT, SIGTERM, CTRL+C, EOF).
    #[arg(long, short)]
    pub foreground: bool,

    /// When running a node in foreground mode, exit the process when receiving EOF on stdin.
    #[arg(long, short, requires = "foreground")]
    pub exit_on_eof: bool,

    /// A flag to determine whether the current foreground node was started as a child process.
    /// This flag is only used internally and should not be set by the user.
    #[arg(hide = true, long, requires = "foreground")]
    pub child_process: bool,
}

/// Wait until it receives a CTRL+C, EOF or a signal to exit
pub async fn wait_for_exit_signal(
    args: &ForegroundArgs,
    opts: &CommandGlobalOpts,
    msg: &str,
) -> miette::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);

    // When running a background node, at this point we don't expect any further output to be written to the
    // parent process, so we close stdin, stdout, and stderr to fully detach the child process.
    // This avoids blocking issues in the parent process when the command is run in environments where the
    // terminal session remains open until all subprocesses exit (e.g. through ssh).
    if args.child_process {
        let _ = nix::unistd::close(0).map_err(std::io::Error::from); // stdin
        let _ = nix::unistd::close(1).map_err(std::io::Error::from); // stdout
        let _ = nix::unistd::close(2).map_err(std::io::Error::from); // stderr
    }

    // Register a handler for SIGINT, SIGTERM, SIGHUP
    {
        let tx = tx.clone();
        let terminal = opts.terminal.clone();
        // To avoid handling multiple CTRL+C signals at the same time
        let mut processed = false;
        let is_child_process = args.child_process;
        ctrlc::set_handler(move || {
            if !processed {
                let _ = tx.blocking_send(());
                info!("Exit signal received");
                if !is_child_process {
                    let _ =
                        terminal.write_line("\n".to_string() + &fmt_warn!("Exit signal received"));
                }
                processed = true
            }
        })
        .expect("Error setting exit signal handler");
    }

    if args.exit_on_eof {
        // Spawn a thread to monitor STDIN for EOF
        {
            let tx = tx.clone();
            let terminal = opts.terminal.clone();
            std::thread::spawn(move || {
                let mut buffer = Vec::new();
                let mut handle = io::stdin().lock();
                if handle.read_to_end(&mut buffer).is_ok() {
                    let _ = tx.blocking_send(());
                    info!("EOF received");
                    let _ = terminal.write_line(fmt_warn!("EOF received"));
                }
            });
        }
    }

    debug!("waiting for exit signal");

    if opts.terminal.is_tty() {
        opts.terminal.write_line("")?;
        opts.terminal.write(fmt_log!("{}", msg))?;
    }

    // Wait for signal SIGINT, SIGTERM, SIGHUP or EOF; or for the tx to be closed.
    rx.recv().await;

    Ok(())
}
