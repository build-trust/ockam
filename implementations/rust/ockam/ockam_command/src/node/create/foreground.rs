use clap::Args;
use std::io;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::time::{sleep, Duration};
use tracing::{debug, info, instrument};

use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::{Address, Context};
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::{
    service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
    NodeManagerWorker, NODEMANAGER_ADDR,
};
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use ockam_core::{route, LOCAL};

use crate::node::CreateCommand;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::CommandGlobalOpts;

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

impl CreateCommand {
    #[instrument(skip_all, fields(node_name = self.name))]
    pub(super) async fn foreground_mode(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let node_name = self.name.clone();
        debug!("creating node in foreground mode");

        if !self.skip_is_running_check
            && opts
                .state
                .get_node(&node_name)
                .await
                .ok()
                .map(|n| n.is_running())
                .unwrap_or(false)
        {
            return Err(miette!(
                "Node {} is already running",
                color_primary(&node_name)
            ));
        }

        let trust_options = opts
            .state
            .retrieve_trust_options(
                &self.trust_opts.project_name,
                &self.trust_opts.authority_identity,
                &self.trust_opts.authority_route,
                &self.trust_opts.credential_scope,
            )
            .await
            .into_diagnostic()?;

        // Create TCP transport
        let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
        let tcp_listener = tcp
            .listen(&self.tcp_listener_address, TcpListenerOptions::new())
            .await
            .into_diagnostic()?;
        info!("TCP listener at {}", tcp_listener.socket_address());

        let _notification_handler = NotificationHandler::start(&opts.state, opts.terminal.clone());

        // Set node_name so that node can isolate its data in the storage from other nodes
        let state = opts.state.clone();

        let node_info = state
            .start_node_with_optional_values(
                &node_name,
                &self.identity,
                &self.trust_opts.project_name,
                Some(&tcp_listener),
            )
            .await?;
        debug!("node info persisted {node_info:?}");

        let http_server_port = if let Some(port) = self.http_server_port {
            Some(port)
        } else if self.enable_http_server {
            if let Some(addr) = node_info.http_server_address() {
                Some(addr.port())
            } else {
                Some(0)
            }
        } else {
            None
        };

        let node_man = InMemoryNode::new(
            ctx,
            NodeManagerGeneralOptions::new(
                state,
                node_name.clone(),
                self.launch_config.is_none(),
                http_server_port,
                true,
            ),
            NodeManagerTransportOptions::new(tcp_listener.flow_control_id().clone(), tcp),
            trust_options,
        )
        .await
        .into_diagnostic()?;
        debug!("in-memory node created");

        let node_manager_worker = NodeManagerWorker::new(Arc::new(node_man));
        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, tcp_listener.flow_control_id());
        ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
            .await
            .into_diagnostic()?;
        debug!("node manager worker started");

        if self.start_services(ctx, &opts).await.is_err() {
            //TODO: Process should terminate on any error during its setup phase,
            //      not just during the start_services.
            //TODO: This sleep here is a workaround on some orchestrated environment,
            //      the lmdb db, that is used for policy storage, fails to be re-opened
            //      if it's still opened from another docker container, where they share
            //      the same pid. By sleeping for a while we let this container be promoted
            //      and the other being terminated, so when restarted it works.  This is
            //      FAR from ideal.
            sleep(Duration::from_secs(10)).await;
            ctx.stop().await.into_diagnostic()?;
            return Err(miette!("Failed to start services"));
        }

        if !self.foreground_args.child_process {
            opts.terminal
                .clone()
                .stdout()
                .plain(self.plain_output(&opts, &node_name).await?)
                .write_line()?;
        }

        drop(_notification_handler);
        self.wait_for_exit_signal(ctx, opts).await
    }

    /// Wait until it receives a CTRL+C, EOF or a signal to exit
    pub async fn wait_for_exit_signal(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);

        // Register a handler for SIGINT, SIGTERM, SIGHUP
        {
            let tx = tx.clone();
            let terminal = opts.terminal.clone();
            // To avoid handling multiple CTRL+C signals at the same time
            let flag = Arc::new(AtomicBool::new(true));
            let is_child_process = self.foreground_args.child_process;
            ctrlc::set_handler(move || {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.blocking_send(());
                    info!("Ctrl+C signal received");
                    if !is_child_process {
                        let _ = terminal.write_line(fmt_warn!("Ctrl+C signal received"));
                    }
                    flag.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            })
            .expect("Error setting Ctrl+C handler");
        }

        if self.foreground_args.exit_on_eof {
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

        if !self.foreground_args.child_process {
            opts.terminal.write_line(&fmt_log!(
                "To exit and stop the Node, please press Ctrl+C\n"
            ))?;
        }

        // Wait for signal SIGINT, SIGTERM, SIGHUP or EOF; or for the tx to be closed.
        rx.recv().await;

        // Clean up and exit
        opts.shutdown();
        let _ = opts.state.stop_node(&self.name, true).await;
        let _ = ctx.stop().await;
        if !self.foreground_args.child_process {
            opts.terminal
                .write_line(fmt_ok!("Node stopped successfully"))?;
        }

        Ok(())
    }

    async fn start_services(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        if let Some(config) = &self.launch_config {
            if let Some(startup_services) = &config.startup_services {
                if let Some(cfg) = startup_services.secure_channel_listener.clone() {
                    if !cfg.disabled {
                        opts.terminal
                            .write_line(fmt_log!("Starting secure-channel listener ..."))?;
                        secure_channel_listener::create_listener(
                            ctx,
                            Address::from((LOCAL, cfg.address)),
                            cfg.authorized_identifiers,
                            cfg.identity,
                            route![],
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
