use std::path::PathBuf;

use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cloud::addon::Addons;
use ockam_api::cloud::project::models::InfluxDBTokenLeaseManagerConfig;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;

use crate::project::addon::check_configuration_completion;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/configure_influxdb/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_influxdb/after_long_help.txt");

/// Configure the InfluxDB Cloud addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureInfluxdbSubcommand {
    #[arg(
        help = docs::about("Ockam Project Name"),
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

    #[arg(
        help = docs::about("Ockam Access Rule for who can use the token lease service"),
        long = "user-access-role",
        id = "user-access-role",
        hide = true,
        value_name = "USER_ACCESS_ROLE",
        value_parser(NonEmptyStringValueParser::new())
    )]
    user_access_role: Option<String>,

    #[arg(
        help = docs::about("Ockam Access Rule for who can manage the token lease service"),
        long = "admin-access-role",
        id = "admin-access-role",
        hide = true,
        value_name = "ADMIN_ACCESS_ROLE",
        value_parser(NonEmptyStringValueParser::new())
    )]
    admin_access_role: Option<String>,
}

impl AddonConfigureInfluxdbSubcommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project addon configure influxdb".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let project_id = opts
            .state
            .projects()
            .get_project_by_name(&self.project_name)
            .await?
            .project_id()
            .to_string();

        let perms = match (&self.permissions, &self.permissions_path) {
            (_, Some(p)) => std::fs::read_to_string(p).into_diagnostic()?,
            (Some(perms), _) => perms.to_string(),
            _ => {
                return Err(miette!(
                    "Permissions JSON is required, supply --permissions or --permissions-path."
                ));
            }
        };

        let config = InfluxDBTokenLeaseManagerConfig::new(
            self.endpoint_url.clone(),
            self.token.clone(),
            self.org_id.clone(),
            perms,
            self.max_ttl_secs,
            self.user_access_role.clone(),
            self.admin_access_role.clone(),
        );

        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let response = controller
            .configure_influxdb_addon(ctx, &project_id, config)
            .await?;
        check_configuration_completion(&opts, ctx, &node, &project_id, &response.operation_id)
            .await?;

        opts.terminal
            .write_line(fmt_ok!("InfluxDB addon configured successfully"))?;
        Ok(())
    }
}
