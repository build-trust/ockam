use crate::node::util::initialize_default_node;
use crate::tcp::outlet::create::CreateCommand as OutletCreateCommand;
use crate::util::parsers::duration_parser;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam::{Address, Context};
use ockam_api::colors::color_primary;
use ockam_api::influxdb::{InfluxDBPortals, LeaseUsage};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};
use std::time::Duration;

/// Create InfluxDB Outlets
#[derive(Clone, Debug, Args)]
pub struct InfluxDBCreateCommand {
    #[command(flatten)]
    pub tcp_outlet: OutletCreateCommand,

    /// The organization ID of the InfluxDB server
    #[arg(long, value_name = "ORG_ID", default_value = "INFLUXDB_ORG_ID")]
    pub org_id: String,

    /// The token to use to connect to the InfluxDB server
    #[arg(long, value_name = "TOKEN", default_value = "INFLUXDB_TOKEN")]
    pub all_access_token: String,

    /// The permissions to grant to new leases
    #[arg(long, value_name = "JSON")]
    pub leased_token_permissions: String,

    /// Share the leases among the clients or use a separate lease for each client
    #[arg(long, default_value = "shared")]
    pub leased_token_strategy: LeaseUsage,

    /// The duration for which a lease is valid
    #[arg(long, value_name = "DURATION", value_parser = duration_parser)]
    pub leased_token_expires_in: Duration,
}

#[async_trait]
impl Command for InfluxDBCreateCommand {
    const NAME: &'static str = "influxdb-outlet create";

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        self = self.parse_args().await?;

        if let Some(pb) = opts.terminal.progress_bar() {
            pb.set_message(format!(
                "Creating a new InfluxDB Outlet to {}...\n",
                color_primary(self.tcp_outlet.to.to_string())
            ));
        }

        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.tcp_outlet.at).await?;
        let outlet_status = node
            .create_influxdb_outlet(
                ctx,
                self.tcp_outlet.to.clone(),
                self.tcp_outlet.tls,
                self.tcp_outlet.from.clone().map(Address::from).as_ref(),
                self.tcp_outlet.allow.clone(),
                self.org_id,
                self.all_access_token,
                self.leased_token_permissions,
                self.leased_token_strategy,
                self.leased_token_expires_in,
            )
            .await?;
        self.tcp_outlet
            .add_outlet_created_journey_event(&opts, &node.node_name(), &outlet_status)
            .await?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "Created a new InfluxDB Outlet in the Node {} at {} bound to {}\n\n",
                    color_primary(node.node_name()),
                    color_primary(&outlet_status.worker_addr),
                    color_primary(&self.tcp_outlet.to)
                ) + &fmt_log!(
                    "You may want to take a look at the {}, {}, {} commands next",
                    color_primary("ockam relay"),
                    color_primary("ockam tcp-inlet"),
                    color_primary("ockam policy")
                ),
            )
            .machine(&outlet_status.worker_addr)
            .json_obj(&outlet_status)?
            .write_line()?;

        Ok(())
    }
}

impl InfluxDBCreateCommand {
    async fn parse_args(mut self) -> miette::Result<Self> {
        if self.org_id == "INFLUXDB_ORG_ID" {
            self.org_id = std::env::var("INFLUXDB_ORG_ID").expect(
                "Pass a value for `--org-id` or export the INFLUXDB_ORG_ID environment variable",
            );
        }
        if self.all_access_token == "INFLUXDB_TOKEN" {
            self.all_access_token = std::env::var("INFLUXDB_TOKEN").expect(
                "Pass a value for `--all-access-token` or export the INFLUXDB_TOKEN environment variable",
            );
        }
        Ok(self)
    }
}
