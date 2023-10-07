use std::path::PathBuf;

use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cloud::addon::Addons;
use ockam_api::cloud::project::InfluxDBTokenLeaseManagerConfig;
use ockam_api::nodes::InMemoryNode;

use crate::project::addon::{check_configuration_completion, get_project_id};
use crate::util::node_rpc;
use crate::{docs, fmt_ok, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/configure_influxdb/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_influxdb/after_long_help.txt");

/// Configure the InfluxDB Cloud addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
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
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonConfigureInfluxdbSubcommand),
) -> miette::Result<()> {
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
    let project_id = get_project_id(&opts.state, project_name.as_str())?;

    let perms = match (permissions, permissions_path) {
        (_, Some(p)) => std::fs::read_to_string(p).into_diagnostic()?,
        (Some(perms), _) => perms,
        _ => {
            return Err(miette!(
                "Permissions JSON is required, supply --permissions or --permissions-path."
            ));
        }
    };

    let config = InfluxDBTokenLeaseManagerConfig::new(
        endpoint_url,
        token,
        org_id,
        perms,
        max_ttl_secs,
        user_access_role,
        admin_access_role,
    );

    let node = InMemoryNode::start(&ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let response = controller
        .configure_influxdb_addon(&ctx, project_id.clone(), config)
        .await?;
    check_configuration_completion(&opts, &ctx, &node, project_id, response.operation_id).await?;

    opts.terminal
        .write_line(&fmt_ok!("InfluxDB addon configured successfully"))?;
    Ok(())
}
