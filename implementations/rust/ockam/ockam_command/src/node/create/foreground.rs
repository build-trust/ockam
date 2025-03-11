use crate::node::create::DEFAULT_NODE_NAME;
use crate::node::CreateCommand;
use crate::service::config::ControlApiNodeResolution;
use crate::util::foreground_args::wait_for_exit_signal;
use crate::CommandGlobalOpts;
use miette::miette;
use miette::IntoDiagnostic;
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::udp::{UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam::{Address, Context};
use ockam_api::cli_state::random_name;
use ockam_api::colors::color_primary;
use ockam_api::control_api::frontend::NodeResolution;
use ockam_api::fmt_log;
use ockam_api::nodes::service::{NodeManagerTransport, SecureChannelType};
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::{
    service::{NodeManagerGeneralOptions, NodeManagerTransportOptions},
    NodeManager, NodeManagerWorker, NODEMANAGER_ADDR,
};
use ockam_api::terminal::notification::NotificationHandler;
use ockam_core::LOCAL;
use ockam_multiaddr::MultiAddr;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, instrument, Level};

impl CreateCommand {
    #[instrument(skip_all, fields(node_name = self.name), level = Level::TRACE)]
    pub(super) async fn foreground_mode(
        &mut self,
        ctx: &Context,
        opts: CommandGlobalOpts,
    ) -> miette::Result<()> {
        self.check_foreground_args(&opts).await?;
        let node_name = self.name.clone();
        debug!("creating node in foreground mode");

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
        let tcp = TcpTransport::get_or_create(ctx).into_diagnostic()?;
        let tcp_listener = tcp
            .listen(&self.tcp_listener_address, TcpListenerOptions::new())
            .await
            .into_diagnostic()?;
        info!("TCP listener at {}", tcp_listener.socket_address());

        // Set node_name so that node can isolate its data in the storage from other nodes
        self.get_or_create_identity(&opts, &self.identity).await?;
        let _notification_handler = if self.foreground_args.child_process {
            // If enabled, the user's terminal would receive notifications
            // from the node after the command exited.
            None
        } else {
            // Enable the notifications only on explicit foreground nodes.
            Some(NotificationHandler::start(
                &opts.state,
                opts.terminal.clone(),
            ))
        };
        let node_info = opts
            .state
            .start_node_with_optional_values(&node_name, &self.identity, Some(&tcp_listener))
            .await?;
        debug!("node info persisted {node_info:?}");

        let udp_options = if self.udp {
            let udp = UdpTransport::get_or_create(ctx).into_diagnostic()?;
            let options = UdpBindOptions::new();
            let flow_control_id = options.flow_control_id();
            udp.bind(
                UdpBindArguments::new().with_bind_address(&self.udp_listener_address)?,
                options,
            )
            .await?;

            Some(NodeManagerTransport::new(flow_control_id, udp))
        } else {
            None
        };

        let in_memory_node = InMemoryNode::new(
            ctx,
            NodeManagerGeneralOptions::new(
                opts.state.clone(),
                node_name.clone(),
                true,
                self.status_endpoint()?,
                true,
            ),
            NodeManagerTransportOptions::new(
                NodeManagerTransport::new(tcp_listener.flow_control_id().clone(), tcp),
                udp_options,
            ),
            trust_options,
        )
        .await
        .into_diagnostic()?;
        debug!("in-memory node created");

        let in_memory_node = Arc::new(in_memory_node);
        let node_manager_worker = NodeManagerWorker::new(in_memory_node.clone());
        ctx.flow_controls()
            .add_consumer(&NODEMANAGER_ADDR.into(), tcp_listener.flow_control_id());
        ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
            .into_diagnostic()?;
        debug!("node manager worker started");

        if self
            .start_services(ctx, in_memory_node.inner_clone(), &opts)
            .await
            .is_err()
        {
            //TODO: Process should terminate on any error during its setup phase,
            //      not just during the start_services.
            //TODO: This sleep here is a workaround on some orchestrated environment,
            //      the lmdb db, that is used for policy storage, fails to be re-opened
            //      if it's still opened from another docker container, where they share
            //      the same pid. By sleeping for a while we let this container be promoted
            //      and the other being terminated, so when restarted it works.  This is
            //      FAR from ideal.
            sleep(Duration::from_secs(10)).await;
            ctx.shutdown_node().await.into_diagnostic()?;
            return Err(miette!("Failed to start services"));
        }

        let node_resources = in_memory_node.get_node_resources().await?;
        opts.terminal
            .clone()
            .to_stdout()
            .plain(self.plain_output(&opts, &node_name).await?)
            .machine(&node_name)
            .json_obj(&node_resources)?
            .write_line()?;

        wait_for_exit_signal(
            &self.foreground_args,
            &opts,
            self.tcp_callback_port,
            "To exit and stop the Node, please press Ctrl+C\n",
        )
        .await?;

        // Clean up and exit
        let _ = opts.state.stop_node(&node_name).await;

        Ok(())
    }

    /// Checks that the arguments specific to foreground nodes are valid.
    async fn check_foreground_args(&mut self, opts: &CommandGlobalOpts) -> miette::Result<()> {
        if !self.skip_is_running_check
            && opts
                .state
                .get_node(&self.name)
                .await
                .ok()
                .map(|n| n.is_running())
                .unwrap_or(false)
        {
            return Err(miette!(
                "Node {} is already running",
                color_primary(&self.name)
            ));
        }

        // return error if trying to create an in-memory node in background mode
        if !self.foreground_args.foreground && opts.state.is_using_in_memory_database()? {
            return Err(miette!("Only foreground nodes can be created in-memory",));
        }

        // if the default node name is used
        if self.name == DEFAULT_NODE_NAME {
            // initialize it with a random name
            self.name = random_name();
        }

        Ok(())
    }

    async fn start_services(
        &self,
        ctx: &Context,
        node_manager: Arc<NodeManager>,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<()> {
        if let Some(config) = &self.services {
            if let Some(startup_services) = &config.services {
                if let Some(cfg) = startup_services.secure_channel_listener.as_ref() {
                    if !cfg.disabled {
                        opts.terminal
                            .write_line(fmt_log!("Starting secure-channel listener ..."))?;
                        node_manager
                            .create_secure_channel_listener(
                                Address::from((LOCAL, cfg.address.clone())),
                                cfg.authorized_identifiers.clone(),
                                cfg.identity.clone(),
                                ctx,
                                SecureChannelType::KeyExchangeAndMessages,
                            )
                            .await?;
                    }
                }

                if let Some(config) = &startup_services.control_api {
                    if config.frontend {
                        let authentication_token = match &config.authentication_token {
                            Some(token) => token.clone(),
                            None => std::env::var("OCKAM_CONTROL_API_AUTHENTICATION_TOKEN")
                                .map_err(|_| {
                                    error!("Frontend Control API needs either `authentication_token` configuration or `OCKAM_CONTROL_API_AUTHENTICATION_TOKEN` environment variable to be set");
                                    miette!("OCKAM_CONTROL_API_AUTHENTICATION_TOKEN not set")
                                })?,
                        };

                        opts.terminal
                            .write_line(fmt_log!("Starting control API Frontend..."))?;

                        let node_resolution = match &config.node_resolution {
                            ControlApiNodeResolution::Relay => {
                                let relay_node = if let Some(node_resolution_relay_node) =
                                    &config.node_resolution_relay_node
                                {
                                    node_resolution_relay_node.parse()?
                                } else if let Ok(default_project_name) =
                                    node_manager.default_project_name().await
                                {
                                    format!("/project/{default_project_name}").parse()?
                                } else {
                                    MultiAddr::default()
                                };
                                NodeResolution::Relay { relay_node }
                            }
                            ControlApiNodeResolution::DirectConnection => {
                                NodeResolution::DirectConnection {
                                    pattern: config.node_resolution_pattern.clone(),
                                    port: config.connection_node_port,
                                }
                            }
                        };

                        node_manager
                            .create_control_api_frontend(
                                ctx,
                                config.http_bind_address,
                                node_resolution,
                                authentication_token,
                                Some(config.backend_policy.clone()),
                            )
                            .await?;
                    }

                    if config.backend {
                        opts.terminal
                            .write_line(fmt_log!("Starting control API Backend..."))?;

                        node_manager.create_control_api_backend(
                            ctx,
                            Some(config.frontend_policy.clone()),
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}
