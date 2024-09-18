use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_api::nodes::service::influxdb_portal_service::InfluxDBPortals;
use tracing::trace;

use crate::influxdb::LeaseUsage;
use crate::node::util::initialize_default_node;
use crate::{Command, CommandGlobalOpts};
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_info, fmt_log, fmt_ok, fmt_warn, ConnectionStatus};
use ockam_core::api::{Reply, Status};
use ockam_multiaddr::MultiAddr;

use crate::tcp::inlet::create::CreateCommand as InletCreateCommand;

/// Create InflucDB Inlets
#[derive(Clone, Debug, Args)]
pub struct InfluxDBCreateCommand {
    #[command(flatten)]
    pub inlet_create_command: InletCreateCommand,

    /// Share the leases among the clients or use a separate lease for each client
    #[arg(long, default_value = "shared")]
    pub lease_usage: LeaseUsage,

    //TODO: this can be given only if LeaseUsage == PerClient
    /// The route to the token leaser service. If not provided,
    /// it's derived from the outlet' route.
    #[arg(long, value_name = "ROUTE")]
    pub lease_issuer_route: Option<MultiAddr>,
}

#[async_trait]
impl Command for InfluxDBCreateCommand {
    const NAME: &'static str = "influxdb-inlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        let inlet_cmd = self.inlet_create_command.parse_args(&opts).await?;

        let mut node = BackgroundNodeClient::create(ctx, &opts.state, &inlet_cmd.at).await?;

        inlet_cmd.timeout.timeout.map(|t| node.set_timeout_mut(t));

        let inlet_status = {
            let pb = opts.terminal.progress_bar();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a InfluxDB Inlet at {}...\n",
                    color_primary(inlet_cmd.from.to_string())
                ));
            }

            //TODO:  do not convert this to string, rather move the LeaseUsage
            // enum definition to ockam_api,  and pass the enum value in the command sent.
            let usage = if self.lease_usage == LeaseUsage::Shared {
                "shared".to_string()
            } else {
                "per-client".to_string()
            };
            loop {
                let result: Reply<InletStatus> = node
                    .create_influxdb_inlet(
                        ctx,
                        &inlet_cmd.from,
                        &inlet_cmd.to(),
                        &inlet_cmd.alias,
                        &inlet_cmd.authorized,
                        &inlet_cmd.allow,
                        inlet_cmd.connection_wait,
                        !inlet_cmd.no_connection_wait,
                        &inlet_cmd.secure_channel_identifier(&opts.state).await?,
                        inlet_cmd.udp,
                        inlet_cmd.no_tcp_fallback,
                        &inlet_cmd.tls_certificate_provider,
                        usage.clone(),
                        self.lease_issuer_route.clone(),
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

                        if inlet_cmd.retry_wait.as_millis() == 0 {
                            return Err(miette!("Failed to create TCP inlet"))?;
                        }

                        if let Some(pb) = pb.as_ref() {
                            pb.set_message(format!(
                                "Waiting for TCP Inlet {} to be available... Retrying momentarily\n",
                                color_primary(&inlet_cmd.to)
                            ));
                        }
                        tokio::time::sleep(inlet_cmd.retry_wait).await
                    }
                }
            }
        };

        let node_name = node.node_name();
        inlet_cmd
            .add_inlet_created_event(&opts, &node_name, &inlet_status)
            .await?;

        let created_message = fmt_ok!(
            "Created a new InfluxDB Inlet in the Node {} bound to {}\n",
            color_primary(&node_name),
            color_primary(inlet_cmd.from.to_string())
        );

        let plain = if inlet_cmd.no_connection_wait {
            created_message + &fmt_log!("It will automatically connect to the TCP Outlet at {} as soon as it is available",
                color_primary(&inlet_cmd.to)
            )
        } else if inlet_status.status == ConnectionStatus::Up {
            created_message
                + &fmt_log!(
                    "sending traffic to the TCP Outlet at {}",
                    color_primary(&inlet_cmd.to)
                )
        } else {
            fmt_warn!(
                "A InfluxDB Inlet was created in the Node {} bound to {} but failed to connect to the TCP Outlet at {}\n",
                color_primary(&node_name),
                 color_primary(inlet_cmd.from.to_string()),
                color_primary(&inlet_cmd.to)
            ) + &fmt_info!("It will retry to connect automatically")
        };

        opts.terminal
            .stdout()
            .plain(plain)
            .machine(inlet_status.bind_addr.to_string())
            .json(serde_json::json!(&inlet_status))
            .write_line()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(
            InfluxDBCreateCommand::NAME,
            &["--token-leaser".to_string(), "/service/leaser".to_string()],
        );
        assert!(cmd.is_ok());
    }
}
