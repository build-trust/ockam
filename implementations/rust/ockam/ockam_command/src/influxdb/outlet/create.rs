use crate::node::util::initialize_default_node;
use crate::tcp::outlet::create::CreateCommand as OutletCreateCommand;
use crate::util::parsers::duration_parser;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::{Address, Context};
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::influxdb::portal::{InfluxDBOutletConfig, LeaseManagerConfig};
use ockam_api::influxdb::InfluxDBPortals;
use ockam_api::nodes::BackgroundNodeClient;
use std::time::Duration;

/// Create InfluxDB Outlets
#[derive(Clone, Debug, Args)]
pub struct InfluxDBCreateCommand {
    #[command(flatten)]
    pub tcp_outlet: OutletCreateCommand,

    #[arg(long, conflicts_with("LeaseManagerConfigArgs"))]
    fixed_token: Option<String>,

    #[clap(flatten)]
    lease_manager_config: Option<LeaseManagerConfigArgs>,
}

#[derive(Args, Clone, Debug)]
#[group(multiple = true)]
pub struct LeaseManagerConfigArgs {
    /// The organization ID of the InfluxDB server
    #[arg(long, value_name = "ORG_ID", default_value = "INFLUXDB_ORG_ID")]
    pub org_id: String,

    /// The token to use to connect to the InfluxDB server
    #[arg(long, value_name = "TOKEN", default_value = "INFLUXDB_TOKEN")]
    pub all_access_token: String,

    /// The permissions to grant to new leases
    #[arg(long, value_name = "JSON")]
    pub leased_token_permissions: String,

    /// The duration for which a lease is valid
    #[arg(long, value_name = "DURATION", value_parser = duration_parser)]
    pub leased_token_expires_in: Duration,
}

#[async_trait]
impl Command for InfluxDBCreateCommand {
    const NAME: &'static str = "influxdb-outlet create";

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;

        let token_config = if let Some(t) = self.fixed_token {
            InfluxDBOutletConfig::OutletWithFixedToken(t)
        } else if let Some(config) = self.lease_manager_config {
            let config = config.parse_args().await?;
            InfluxDBOutletConfig::StartLeaseManager(LeaseManagerConfig::new(
                config.org_id,
                config.all_access_token,
                config.leased_token_permissions,
                config.leased_token_expires_in,
            ))
        } else {
            return Err(miette!(
                "Either configure a fixed-token, or the arguments to handle token leases"
            ))?;
        };

        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.tcp_outlet.at).await?;
        let outlet_status = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a new InfluxDB Outlet to {}...\n",
                    color_primary(self.tcp_outlet.to.to_string())
                ));
            }
            node.create_influxdb_outlet(
                ctx,
                self.tcp_outlet.to.clone(),
                self.tcp_outlet.tls,
                self.tcp_outlet.from.clone().map(Address::from).as_ref(),
                self.tcp_outlet.allow.clone(),
                token_config,
            )
            .await?
        };
        self.tcp_outlet
            .add_outlet_created_journey_event(&opts, &node.node_name(), &outlet_status)
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Created a new InfluxDB Outlet in the Node {} at {} bound to {}\n\n",
                color_primary(node.node_name()),
                color_primary(&outlet_status.worker_addr),
                color_primary(&self.tcp_outlet.to)
            ))
            .machine(&outlet_status.worker_addr)
            .json_obj(&outlet_status)?
            .write_line()?;
        Ok(())
    }
}

impl LeaseManagerConfigArgs {
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
