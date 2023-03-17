use std::path::PathBuf;

use anyhow::{anyhow, Context as _};
use clap::builder::NonEmptyStringValueParser;
use clap::Args;

use ockam::Context;

use ockam_api::cloud::project::{InfluxDBTokenLeaseManagerConfig, Project};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;

use crate::node::util::delete_embedded_node;
use crate::project::addon::base_endpoint;
use crate::project::config;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;

use crate::util::{api, exitcode, node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts, Result};

const INFLUXDB_HELP_DETAIL: &str = r#"
About:
    InfluxDB addon allows you to create, store and retrieve InfluxDB Tokens with expiry times.

Examples:
    Examples of how to configure and use the InfluxDB Cloud addon can be found within the example documentation.
    https://docs.ockam.io/guides/examples/influxdb-cloud-token-lease-management
"#;

/// Configure the InfluxDB Cloud addon for a project
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(INFLUXDB_HELP_DETAIL))]
pub struct AddonConfigureInfluxdbSubcommand {
    /// Ockam Project Name
    #[arg(
        long = "project",
        id = "project",
        value_name = "PROJECT_NAME",
        default_value = "default",
        value_parser(NonEmptyStringValueParser::new())
    )]
    project_name: String,

    /// Url of the InfluxDB instance
    #[arg(
        short,
        long,
        id = "endpoint",
        value_name = "ENDPOINT_URL",
        value_parser(NonEmptyStringValueParser::new())
    )]
    endpoint_url: String,

    /// InfluxDB Token with permissions to perform CRUD token operations
    #[arg(
        short,
        long,
        id = "token",
        value_name = "INFLUXDB_TOKEN",
        value_parser(NonEmptyStringValueParser::new())
    )]
    token: String,

    /// InfluxDB Organization ID
    #[arg(
        short,
        long,
        id = "org_id",
        value_name = "ORGANIZATION_ID",
        value_parser(NonEmptyStringValueParser::new())
    )]
    org_id: String,

    /// InfluxDB Permissions as a JSON String
    /// https://docs.influxdata.com/influxdb/v2.0/api/#operation/PostAuthorizations
    #[arg(
    long = "permissions",
    group = "permissions_group",  //looks like it can't be named the same than an existing field
    value_name = "PERMISSIONS_JSON",
    value_parser(NonEmptyStringValueParser::new())
    )]
    permissions: Option<String>,

    /// InfluxDB Permissions JSON PATH. Use either this or --permissions
    #[arg(
        long = "permissions-path",
        group = "permissions_group",
        value_name = "PERMISSIONS_JSON_PATH"
    )]
    permissions_path: Option<PathBuf>,

    /// Max TTL of Tokens within the Lease Manager [Defaults to 3 Hours]
    #[arg(
        long = "max-ttl",
        id = "max_ttl",
        value_name = "MAX_TTL_SECS",
        default_value = "10800"
    )]
    max_ttl_secs: i32,

    /// Ockam Access Rule for who can use the token lease service
    #[arg(
        long = "user-access-role",
        id = "user-access-role",
        hide = true,
        value_name = "USER_ACCESS_ROLE",
        value_parser(NonEmptyStringValueParser::new())
    )]
    user_access_role: Option<String>,

    /// Ockam Access Rule for who can manage the token lease service
    #[arg(
        long = "adamin-access-role",
        id = "admin-access-role",
        hide = true,
        value_name = "ADMIN_ACCESS_ROLE",
        value_parser(NonEmptyStringValueParser::new())
    )]
    admin_access_role: Option<String>,
}

impl AddonConfigureInfluxdbSubcommand {
    pub fn run(self, opts: CommandGlobalOpts, cloud_opts: CloudOpts) {
        node_rpc(run_impl, (opts, cloud_opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cloud_opts, cmd): (
        CommandGlobalOpts,
        CloudOpts,
        AddonConfigureInfluxdbSubcommand,
    ),
) -> Result<()> {
    let controller_route = &cloud_opts.route();
    let AddonConfigureInfluxdbSubcommand {
        project_name,
        endpoint_url,
        token,
        org_id,
        permissions,
        permissions_path,
        max_ttl_secs,
        user_access_role,
        admin_access_role,
    } = cmd;

    let mut rpc = Rpc::embedded(&ctx, &opts).await?;
    let perms = match (permissions, permissions_path) {
        (_, Some(p)) => std::fs::read_to_string(p)?,
        (Some(perms), _) => perms,
        _ => {
            return Err(crate::error::Error::new(
                exitcode::IOERR,
                anyhow!(
                    "Permissions JSON is required, supply --permissions or --permissions-path."
                ),
            ));
        }
    };

    let body = InfluxDBTokenLeaseManagerConfig::new(
        endpoint_url,
        token,
        org_id,
        perms,
        max_ttl_secs,
        user_access_role,
        admin_access_role,
    );

    let add_on_id = "influxdb_token_lease_manager";
    let endpoint = format!(
        "{}/{}",
        base_endpoint(&opts.config.lookup(), &project_name)?,
        add_on_id
    );
    let req = Request::put(endpoint).body(CloudRequestWrapper::new(
        body,
        controller_route,
        None::<CowStr>,
    ));

    rpc.request(req).await?;
    rpc.is_ok()?;
    println!("InfluxDB addon enabled");

    // Wait until project is ready again
    println!("Reconfiguring project (this can take a few minutes) ...");
    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

    let project_id =
        config::get_project(&opts.config, &project_name).context("project not found in lookup")?;

    // Give the sever ~20 seconds
    // in intervals of 5s for the project to be available.
    for _ in 0..4 {
        rpc.request(api::project::show(&project_id, controller_route))
            .await?;
        let project: Project = match rpc.parse_response() {
            Ok(p) => p,
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        check_project_readiness(&ctx, &opts, &cloud_opts, rpc.node_name(), None, project).await?;
    }
    println!("InfluxDB addon configured successfully");
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}
