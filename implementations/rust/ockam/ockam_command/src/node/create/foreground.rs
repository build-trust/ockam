use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::time::{sleep, Duration};
use tracing::{debug, instrument};

use ockam::{Address, AsyncTryClone, TcpListenerOptions};
use ockam::{Context, TcpTransport};
use ockam_api::logs::TracingGuard;
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::{
    service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
    NodeManagerWorker, NODEMANAGER_ADDR,
};
use ockam_core::{route, LOCAL};

use crate::fmt_ok;
use crate::node::CreateCommand;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::{shutdown, CommandGlobalOpts};

impl CreateCommand {
    #[instrument(skip_all, fields(node_name = self.node_name))]
    pub(super) async fn foreground_mode(
        &self,
        ctx: &Context,
        opts: CommandGlobalOpts,
        tracing_guard: Option<Arc<TracingGuard>>,
    ) -> miette::Result<()> {
        self.guard_node_is_not_already_running(&opts).await?;

        let node_name = self.node_name.clone();
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
        let options = TcpListenerOptions::new();
        let listener = tcp
            .listen(&self.tcp_listener_address, options)
            .await
            .into_diagnostic()?;

        debug!(
            "set the node {node_name} listener address to {:?}",
            listener.socket_address()
        );

        // Set node_name so that node can isolate its data in the storage from other nodes
        let mut state = opts.state.clone();
        state.set_node_name(node_name.clone());

        let node_info = state
            .start_node_with_optional_values(
                &node_name,
                &self.identity,
                &self.trust_opts.project_name,
                Some(&listener),
            )
            .await?;
        debug!("created node {node_info:?}");

        let trust_options = opts
            .state
            .retrieve_trust_options(
                &self.trust_opts.project_name,
                &self.trust_opts.authority_identity,
                &self.trust_opts.authority_route,
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
            NodeManagerTransportOptions::new(
                listener.flow_control_id().clone(),
                tcp.async_try_clone().await.into_diagnostic()?,
            ),
            trust_options,
        )
        .await
        .into_diagnostic()?;
        let node_manager_worker = NodeManagerWorker::new(Arc::new(node_man));

        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
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

        if let Some(tracing_guard) = tracing_guard {
            tracing_guard.force_flush();
        };

        // Create a channel for communicating back to the main thread
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        shutdown::wait(
            opts.terminal.clone(),
            self.exit_on_eof,
            opts.global_args.quiet,
            tx,
            &mut rx,
        )
        .await?;

        // Try to stop node; it might have already been stopped or deleted (e.g. when running `node delete --all`)
        opts.state.stop_node(&node_name, true).await?;
        ctx.stop().await.into_diagnostic()?;
        opts.terminal
            .write_line(fmt_ok!("Node stopped successfully"))
            .unwrap();

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
