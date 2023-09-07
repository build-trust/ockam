use crate::{Terminal, TerminalStream};
use colorful::Colorful;
use console::Term;
use std::io;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::info;

/// Waits for CTRL+C, EOF or a signal to exit, can provide extra shutdown events by
/// sending a message through the channel
pub async fn wait(
    terminal: Terminal<TerminalStream<Term>>,
    exit_on_eof: bool,
    quiet: bool,
    tx: Sender<()>,
    rx: &mut Receiver<()>,
) -> miette::Result<bool> {
    // Register a handler for SIGINT, SIGTERM, SIGHUP
    {
        let tx = tx.clone();
        let terminal = terminal.clone();
        // avoid printing CTRL+C multiple times
        let flag = Arc::new(AtomicBool::new(true));
        ctrlc::set_handler(move || {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = tx.blocking_send(());
                info!("Ctrl+C signal received");
                if !quiet {
                    let _ = terminal.write_line(
                        format!("{} Ctrl+C signal received", "!".light_yellow()).as_str(),
                    );
                }
                flag.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        })
        .expect("Error setting Ctrl+C handler");
    }

    if exit_on_eof {
        // Spawn a thread to monitor STDIN for EOF
        {
            let tx = tx.clone();
            let terminal = terminal.clone();
            std::thread::spawn(move || {
                let mut buffer = Vec::new();
                let mut handle = io::stdin().lock();
                handle
                    .read_to_end(&mut buffer)
                    .expect("Error reading from stdin");
                let _ = tx.blocking_send(());
                info!("EOF received");
                if !quiet {
                    let _ = terminal
                        .write_line(format!("{} EOF received", "!".light_yellow()).as_str());
                }
            });
        }
    }

    // Shutdown on SIGINT, SIGTERM, SIGHUP or EOF
    Ok(rx.recv().await.is_some())
}
