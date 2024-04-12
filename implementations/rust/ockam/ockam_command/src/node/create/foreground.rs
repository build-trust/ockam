use std::io;
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::time::{sleep, Duration};
use tracing::{debug, info, instrument};

use ockam::{Address, TcpListenerOptions};
use ockam::{Context, TcpTransport};
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::{
    service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
    NodeManagerWorker, NODEMANAGER_ADDR,
};
use ockam_api::{fmt_ok, fmt_warn};
use ockam_core::{route, LOCAL};

use crate::node::CreateCommand;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::CommandGlobalOpts;

impl CreateCommand {
    #[instrument(skip_all, fields(node_name = self.name))]
    pub(super) async fn foreground_mode(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        self.guard_node_is_not_already_running(&opts).await?;

        let node_name = self.name.clone();
        debug!("create node {node_name} in foreground mode");

        if opts
            .state
            .get_node(&node_name)
            .await
            .ok()
            .map(|n| n.is_running())
            .unwrap_or(false)
        {
            return Err(miette!("Node {} is already running", &node_name));
        };

        let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
        let tcp_listener = tcp
            .listen(&self.tcp_listener_address, TcpListenerOptions::new())
            .await
            .into_diagnostic()?;

        debug!(
            "set the node {node_name} listener address to {:?}",
            tcp_listener.socket_address()
        );

        // Set node_name so that node can isolate its data in the storage from other nodes
        let mut state = opts.state.clone();
        state.set_node_name(&node_name);

        let node_info = state
            .start_node_with_optional_values(
                &node_name,
                &self.identity,
                &self.trust_opts.project_name,
                Some(&tcp_listener),
            )
            .await?;
        debug!("created node {node_info:?}");

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

        let node_man = InMemoryNode::new(
            ctx,
            NodeManagerGeneralOptions::new(
                state,
                node_name.clone(),
                self.launch_config.is_none(),
                true,
            ),
            NodeManagerTransportOptions::new(tcp_listener.flow_control_id().clone(), tcp),
            trust_options,
        )
        .await
        .into_diagnostic()?;
        let node_manager_worker = NodeManagerWorker::new(Arc::new(node_man));

        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, tcp_listener.flow_control_id());
        ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
            .await
            .into_diagnostic()?;

        if let Some(config) = &self.launch_config {
            if start_services(ctx, config).await.is_err() {
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
        }

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
            ctrlc::set_handler(move || {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.blocking_send(());
                    info!("Ctrl+C signal received");
                    if !opts.global_args.quiet {
                        let _ = terminal.write_line(fmt_warn!("Ctrl+C signal received"));
                    }
                    flag.store(false, std::sync::atomic::Ordering::Relaxed);
                }
            })
            .expect("Error setting Ctrl+C handler");
        }

        if self.exit_on_eof {
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

        // Wait for signal SIGINT, SIGTERM, SIGHUP or EOF; or for the tx to be closed.
        rx.recv().await;

        // Clean up and exit
        opts.shutdown();
        let _ = opts.state.stop_node(&self.name, true).await;
        let _ = ctx.stop().await;
        opts.terminal
            .write_line(fmt_ok!("Node stopped successfully"))?;

        Ok(())
    }
}

async fn start_services(ctx: &Context, cfg: &Config) -> miette::Result<()> {
    let config = {
        if let Some(sc) = &cfg.startup_services {
            sc.clone()
        } else {
            return Ok(());
        }
    };

    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let identity = cfg.identity;
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, identity, route![]).await?;
        }
    }

    Ok(())
}
