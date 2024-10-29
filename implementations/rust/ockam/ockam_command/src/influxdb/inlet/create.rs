use crate::node::util::initialize_default_node;
use crate::tcp::inlet::create::CreateCommand as InletCreateCommand;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::influxdb::{InfluxDBPortals, LeaseUsage};
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn, ConnectionStatus};
use ockam_core::api::{Reply, Status};
use ockam_multiaddr::MultiAddr;
use tracing::trace;

/// Create InfluxDB Inlets
#[derive(Clone, Debug, Args)]
pub struct InfluxDBCreateCommand {
    #[command(flatten)]
    pub tcp_inlet: InletCreateCommand,

    /// Share the leases among the clients or use a separate lease for each client
    #[arg(long, default_value = "per-client")]
    pub leased_token_strategy: LeaseUsage,

    /// The route to the lease issuer service.
    /// Only applicable if `lease-token-strategy` is set to `per-client`.
    /// If not provided, it's derived from the outlet route.
    #[arg(long, value_name = "ROUTE")]
    pub lease_manager_route: Option<MultiAddr>,
}

#[async_trait]
impl Command for InfluxDBCreateCommand {
    const NAME: &'static str = "influxdb-inlet create";

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        self = self.parse_args(&opts).await?;

        let mut node = BackgroundNodeClient::create(ctx, &opts.state, &self.tcp_inlet.at).await?;
        self.tcp_inlet
            .timeout
            .timeout
            .map(|t| node.set_timeout_mut(t));

        let inlet_status = {
            let pb = opts.terminal.progress_bar();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a InfluxDB Inlet at {}...\n",
                    color_primary(&self.tcp_inlet.from)
                ));
            }

            loop {
                let result: Reply<InletStatus> = node
                    .create_influxdb_inlet(
                        ctx,
                        &self.tcp_inlet.from,
                        &self.tcp_inlet.to(),
                        self.tcp_inlet.alias.as_ref().expect("The `alias` argument should be set to its default value if not provided"),
                        &self.tcp_inlet.authorized,
                        &self.tcp_inlet.allow,
                        self.tcp_inlet.connection_wait,
                        !self.tcp_inlet.no_connection_wait,
                        &self
                            .tcp_inlet
                            .secure_channel_identifier(&opts.state)
                            .await?,
                        self.tcp_inlet.udp,
                        self.tcp_inlet.no_tcp_fallback,
                        &self.tcp_inlet.tls_certificate_provider,
                        self.leased_token_strategy.clone(),
                        self.lease_manager_route.clone(),
                    )
                    .await?;

                match result {
                    Reply::Successful(inlet_status) => {
                        break inlet_status;
                    }
                    Reply::Failed(_, s) => {
                        if let Some(status) = s {
                            if status == Status::BadRequest {
                                Err(miette!("Bad request when creating an inlet"))?
                            }
                        };
                        trace!("the inlet creation returned a non-OK status: {s:?}");

                        if self.tcp_inlet.retry_wait.as_millis() == 0 {
                            return Err(miette!("Failed to create TCP inlet"))?;
                        }

                        if let Some(pb) = pb.as_ref() {
                            pb.set_message(format!(
                                "Waiting for TCP Inlet {} to be available... Retrying momentarily\n",
                                color_primary(&self.tcp_inlet.to)
                            ));
                        }
                        tokio::time::sleep(self.tcp_inlet.retry_wait).await
                    }
                }
            }
        };

        let node_name = node.node_name();
        self.tcp_inlet
            .add_inlet_created_event(&opts, &node_name, &inlet_status)
            .await?;

        let created_message = fmt_ok!(
            "Created a new InfluxDB Inlet in the Node {} bound to {}\n",
            color_primary(&node_name),
            color_primary(&self.tcp_inlet.from)
        );

        let plain = if self.tcp_inlet.no_connection_wait {
            created_message + &fmt_log!("It will automatically connect to the TCP Outlet at {} as soon as it is available",
                color_primary(&self.tcp_inlet.to)
            )
        } else if inlet_status.status == ConnectionStatus::Up {
            created_message
                + &fmt_log!(
                    "sending traffic to the TCP Outlet at {}",
                    color_primary(&self.tcp_inlet.to)
                )
        } else {
            fmt_warn!(
                "A InfluxDB Inlet was created in the Node {} bound to {} but failed to connect to the TCP Outlet at {}\n",
                color_primary(&node_name),
                 color_primary(self.tcp_inlet.from.to_string()),
                color_primary(&self.tcp_inlet.to)
            ) + &fmt_info!("It will retry to connect automatically")
        };

        opts.terminal
            .stdout()
            .plain(plain)
            .machine(inlet_status.bind_addr.to_string())
            .json_obj(&inlet_status)?
            .write_line()?;

        Ok(())
    }
}

impl InfluxDBCreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        self.tcp_inlet = self.tcp_inlet.parse_args(opts).await?;
        if self
            .lease_manager_route
            .as_ref()
            .is_some_and(|_| self.leased_token_strategy == LeaseUsage::Shared)
        {
            Err(miette!(
                "lease-manager-route argument requires leased-token-strategy=per-client"
            ))?
        };
        Ok(self)
    }
}
