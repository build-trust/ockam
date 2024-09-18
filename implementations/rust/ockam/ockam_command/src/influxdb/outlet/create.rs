use std::time::Duration;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use ockam::{Address, Context};
use ockam_api::nodes::service::influxdb_portal_service::InfluxDBPortals;

use crate::influxdb::LeaseUsage;
use crate::node::util::initialize_default_node;
use crate::util::parsers::duration_parser;
use crate::{Command, CommandGlobalOpts};
use ockam_api::colors::color_primary;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};

use crate::tcp::outlet::create::CreateCommand as OutletCreateCommand;

/// Create InflucDB Outlets
#[derive(Clone, Debug, Args)]
pub struct InfluxDBCreateCommand {
    #[command(flatten)]
    pub outlet_create_command: OutletCreateCommand,

    /// The organization ID of the InfluxDB server
    #[arg(long, value_name = "ORG_ID", default_value_t = default_influxdb_org_id())]
    pub influxdb_org_id: String,

    /// The token to use to connect to the InfluxDB server
    #[arg(long, value_name = "TOKEN", default_value_t = default_influxdb_token())]
    pub influxdb_token: String,

    /// The permissions to grant to new leases
    #[arg(long, value_name = "JSON")]
    pub lease_permissions: String,

    /// Share the leases among the clients or use a separate lease for each client
    #[arg(long, default_value = "shared")]
    pub lease_usage: LeaseUsage,

    /// The duration for which a lease is valid
    #[arg(long = "expires-in", value_name = "DURATION", value_parser = duration_parser)]
    pub expires_in: Duration,
}

fn default_influxdb_org_id() -> String {
    //TODO: FIXME:  as is, it doesn't work when using trom a node config yaml. 
    //              the cmd fails to parse even when the parameter is given.
    "TODO".to_string()
    /*std::env::var("INFLUXDB_ORG_ID").expect(
        "Pass a value for `--influxdb-org-id` or export the INFLUXDB_ORG_ID environment variable",
    ) */
}

fn default_influxdb_token() -> String {
    "TODO".to_string()
    /*std::env::var("INFLUXDB_TOKEN").expect(
        "Pass a value for `--influxdb-token` or export the INFLUXDB_TOKEN environment variable",
    ) */
}

#[async_trait]
impl Command for InfluxDBCreateCommand {
    const NAME: &'static str = "influxdb-outlet create";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let outlet_cmd = self.outlet_create_command;

        if let Some(pb) = opts.terminal.progress_bar() {
            pb.set_message(format!(
                "Creating a new InfluxDB Outlet to {}...\n",
                color_primary(outlet_cmd.to.to_string())
            ));
        }
        let node = BackgroundNodeClient::create(ctx, &opts.state, &outlet_cmd.at).await?;
        let node_name = node.node_name();
        let usage = if self.lease_usage == LeaseUsage::Shared {
            "shared".to_string()
        } else {
            "per-client".to_string()
        };
        let outlet_status = node
            .create_influxdb_outlet(
                ctx,
                outlet_cmd.to.clone(),
                outlet_cmd.tls,
                outlet_cmd.from.clone().map(Address::from).as_ref(),
                outlet_cmd.allow.clone(),
                self.influxdb_org_id,
                self.influxdb_token,
                self.lease_permissions,
                usage, //self.lease_usage,
                self.expires_in,
            )
            .await?;
        outlet_cmd
            .add_outlet_created_journey_event(&opts, &node_name, &outlet_status)
            .await?;

        let worker_addr = outlet_status.worker_address().into_diagnostic()?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "Created a new InfluxDB Outlet in the Node {} at {} bound to {}\n\n",
                    color_primary(&node_name),
                    color_primary(worker_addr.to_string()),
                    color_primary(outlet_cmd.to.to_string())
                ) + &fmt_log!(
                    "You may want to take a look at the {}, {}, {} commands next",
                    color_primary("ockam relay"),
                    color_primary("ockam tcp-inlet"),
                    color_primary("ockam policy")
                ),
            )
            .machine(worker_addr)
            .json(serde_json::to_string(&outlet_status).into_diagnostic()?)
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
            &[
                "--to".to_string(),
                "127.0.0.1:5000".to_string(),
                "--token-leaser".to_string(),
                "/service/test".to_string(),
            ],
        );
        assert!(cmd.is_ok());
    }
}
