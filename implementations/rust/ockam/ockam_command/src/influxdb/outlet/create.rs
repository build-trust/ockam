use crate::node::util::initialize_default_node;
use crate::util::parsers::duration_parser;
use crate::util::parsers::hostname_parser;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::builder::FalseyValueParser;
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::transport::SchemeHostnamePort;
use ockam::{Address, Context};
use ockam_abac::PolicyExpression;
use ockam_api::address::extract_address_value;
use ockam_api::colors::color_primary;
use ockam_api::influxdb::portal::{InfluxDBOutletConfig, LeaseManagerConfig};
use ockam_api::influxdb::InfluxDBPortals;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok, fmt_warn};
use std::time::Duration;

/// Create InfluxDB Outlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Address of your InfluxDB Outlet, which is part of a route used in other commands.
    /// This unique address identifies the InfluxDB Outlet worker on the Node on your local machine.
    /// Examples are `/service/my-outlet` or `my-outlet`.
    /// If not provided, `outlet` will be used, or a random address will be generated if `outlet` is taken.
    /// You will need this address when creating a InfluxDB Inlet using `ockam influxdb-inlet create`.
    #[arg(value_parser = extract_address_value)]
    pub name: Option<String>,

    /// Address where your InfluxDB server is running, in the format `<scheme>://<hostname>:<port>`.
    /// At least the port must be provided. The default scheme is `tcp` and the default hostname is `127.0.0.1`.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", value_parser = hostname_parser)]
    pub to: SchemeHostnamePort,

    /// [DEPRECATED] Use the `tls` scheme in the `--from` argument.
    #[arg(long, display_order = 900, id = "BOOLEAN")]
    pub tls: bool,

    /// Alternative to the <NAME> positional argument.
    /// Address of your InfluxDB Outlet, which is part of a route used in other commands.
    #[arg(long, display_order = 902, id = "OUTLET_ADDRESS", value_parser = extract_address_value)]
    pub from: Option<String>,

    /// Your InfluxDB Outlet will be created on this node. If you don't provide it, the default
    /// node will be used
    #[arg(long, display_order = 903, id = "NODE_NAME", value_parser = extract_address_value)]
    pub at: Option<String>,

    /// Policy expression that will be used for access control to the InfluxDB Outlet.
    /// If you don't provide it, the policy set for the "tcp-outlet" resource type will be used.
    ///
    /// You can check the fallback policy with `ockam policy show --resource-type tcp-outlet`.
    #[arg(
        long,
        visible_alias = "expression",
        display_order = 904,
        id = "POLICY_EXPRESSION"
    )]
    pub allow: Option<PolicyExpression>,

    /// Use eBPF and RawSocket to access TCP packets instead of TCP data stream.
    /// If `OCKAM_PRIVILEGED` env variable is set to 1, this argument will be `true`.
    #[arg(long, env = "OCKAM_PRIVILEGED", value_parser = FalseyValueParser::default(), hide = true)]
    pub privileged: bool,

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
impl Command for CreateCommand {
    const NAME: &'static str = "influxdb-outlet create";

    async fn async_run(mut self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        initialize_default_node(ctx, &opts).await?;
        let cmd = self.parse_args(&opts).await?;

        let token_config = if let Some(t) = cmd.fixed_token {
            InfluxDBOutletConfig::OutletWithFixedToken(t)
        } else if let Some(config) = cmd.lease_manager_config {
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

        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        let outlet_status = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!(
                    "Creating a new InfluxDB Outlet to {}...\n",
                    color_primary(cmd.to.to_string())
                ));
            }
            node.create_influxdb_outlet(
                ctx,
                cmd.to.clone().into(),
                cmd.tls || cmd.to.is_tls(),
                cmd.name.clone().map(Address::from).as_ref(),
                cmd.allow.clone(),
                token_config,
            )
            .await?
        };

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Created a new InfluxDB Outlet in the Node {} at {} bound to {}\n\n",
                color_primary(node.node_name()),
                color_primary(&outlet_status.worker_addr),
                color_primary(&cmd.to)
            ))
            .machine(&outlet_status.worker_addr)
            .json_obj(&outlet_status)?
            .write_line()?;
        Ok(())
    }
}

impl CreateCommand {
    async fn parse_args(mut self, opts: &CommandGlobalOpts) -> miette::Result<Self> {
        if let Some(from) = self.from.as_ref() {
            if self.name.is_some() {
                opts.terminal.write_line(
                    fmt_warn!("The <NAME> argument is being overridden by the --from flag")
                        + &fmt_log!("Consider using either the <NAME> argument or the --from flag"),
                )?;
            }
            self.name = Some(from.clone());
        }

        Ok(self)
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
