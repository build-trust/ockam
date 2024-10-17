use std::process::exit;
use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::time::{sleep, Duration};
use tracing::{debug, info, instrument};

use crate::node::show::is_node_up;
use crate::node::CreateCommand;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::util::foreground_args::wait_for_exit_signal;
use crate::CommandGlobalOpts;
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::udp::UdpTransport;
use ockam::{Address, Context};
use ockam_api::colors::color_primary;
use ockam_api::nodes::{
    service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
    NodeManagerWorker, NODEMANAGER_ADDR,
};
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode};
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::{route, LOCAL};

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

        // Set node_name so that node can isolate its data in the storage from other nodes
        self.get_or_create_identity(&opts, &self.identity).await?;
        let _notification_handler = NotificationHandler::start(&opts.state, opts.terminal.clone());
        let node_info = opts
            .state
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
        } else if self.http_server {
            if let Some(addr) = node_info.http_server_address() {
                Some(addr.port())
            } else {
                Some(0)
            }
        } else {
            None
        };

        let udp_transport = if self.udp {
            Some(UdpTransport::create(ctx).await.into_diagnostic()?)
        } else {
            None
        };

        let node_man = InMemoryNode::new(
            ctx,
            NodeManagerGeneralOptions::new(
                opts.state.clone(),
                node_name.clone(),
                self.launch_config.is_none(),
                http_server_port,
                true,
            ),
            NodeManagerTransportOptions::new(
                tcp_listener.flow_control_id().clone(),
                tcp,
                udp_transport,
            ),
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
        wait_for_exit_signal(
            &self.foreground_args,
            &opts,
            "To exit and stop the Node, please press Ctrl+C\n",
        )
        .await?;

        // Clean up and exit
        let _ = ctx.stop().await;
        let _ = opts.state.stop_node(&node_name).await;
        if self.foreground_args.child_process {
            opts.shutdown();
            exit(0);
        } else {
            opts.terminal
                .write_line(fmt_ok!("Node stopped successfully"))?;
            Ok(())
        }
    }

    async fn start_services(&self, ctx: &Context, opts: &CommandGlobalOpts) -> miette::Result<()> {
        // Wait until the node is fully started
        let mut node =
            BackgroundNodeClient::create(ctx, &opts.state, &Some(self.name.clone())).await?;
        if !is_node_up(ctx, &mut node, true).await? {
            return Err(miette!(
                "Couldn't start services because the node is not up"
            ));
        }

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
