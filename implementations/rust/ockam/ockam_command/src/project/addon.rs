use core::fmt::Write;
use std::path::PathBuf;

use anyhow::Context as _;
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};

use ockam::Context;
use ockam_api::cloud::addon::Addon;
use ockam_api::cloud::project::OktaConfig;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::delete_embedded_node;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{node_rpc, Rpc};
use crate::{help, CommandGlobalOpts};

const HELP_DETAIL: &str = "";

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide(), help_template = help::template(HELP_DETAIL))]
pub struct AddonCommand {
    #[command(subcommand)]
    subcommand: AddonSubcommand,

    #[command(flatten)]
    cloud_opts: CloudOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum AddonSubcommand {
    List {
        /// Project name
        #[arg(
            long = "project",
            id = "project",
            value_name = "PROJECT_NAME",
            value_parser(NonEmptyStringValueParser::new())
        )]
        project_name: String,
    },
    Disable {
        /// Project name
        #[arg(
            long = "project",
            id = "project",
            value_name = "PROJECT_NAME",
            value_parser(NonEmptyStringValueParser::new())
        )]
        project_name: String,

        /// Addon id/name
        #[arg(
            long = "addon",
            id = "addon",
            value_name = "ADDON_ID",
            value_parser(NonEmptyStringValueParser::new())
        )]
        addon_id: String,
    },
    #[command(subcommand)]
    Configure(ConfigureAddonCommand),
}

#[derive(Clone, Debug, Subcommand)]
pub enum ConfigureAddonCommand {
    Okta {
        /// Project name
        #[arg(
            long = "project",
            id = "project",
            value_name = "PROJECT_NAME",
            value_parser(NonEmptyStringValueParser::new())
        )]
        project_name: String,

        /// Plugin tenant URL
        #[arg(
            long,
            id = "tenant",
            value_name = "TENANT",
            value_parser(NonEmptyStringValueParser::new())
        )]
        tenant: String,

        /// Certificate. Use either this or --cert-path
        #[arg(
            long = "cert",
            group = "cert",
            value_name = "CERTIFICATE",
            value_parser(NonEmptyStringValueParser::new())
        )]
        certificate: Option<String>,

        /// Certificate file path. Use either this or --cert
        #[arg(long = "cert-path", group = "cert", value_name = "CERTIFICATE_PATH")]
        certificate_path: Option<PathBuf>,

        /// Okta Client ID.
        #[arg(
            long,
            id = "client_id",
            value_name = "CLIENT_ID",
            value_parser(NonEmptyStringValueParser::new())
        )]
        client_id: String,

        /// Attributes names to copy from Okta userprofile into Ockam credential.
        #[arg(long = "attr", value_name = "ATTRIBUTE")]
        attributes: Vec<String>,
    },
}

impl AddonCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddonCommand),
) -> crate::Result<()> {
    let controller_route = &cmd.cloud_opts.route();
    let mut rpc = Rpc::embedded(&ctx, &opts).await?;

    let base_endpoint = |project_name: &str| -> crate::Result<String> {
        let lookup = opts.config.lookup();
        let project_id = &lookup
            .get_project(project_name)
            .context(format!(
                "Failed to get project {} from config lookup",
                project_name
            ))?
            .id;
        Ok(format!("{project_id}/addons"))
    };

    match cmd.subcommand {
        AddonSubcommand::List { project_name } => {
            let req = Request::get(base_endpoint(&project_name)?)
                .body(CloudRequestWrapper::bare(controller_route));
            rpc.request(req).await?;
            rpc.parse_and_print_response::<Vec<Addon>>()?;
        }
        AddonSubcommand::Disable {
            project_name,
            addon_id,
        } => {
            let endpoint = format!("{}/{}", base_endpoint(&project_name)?, addon_id);
            let req = Request::delete(endpoint).body(CloudRequestWrapper::bare(controller_route));
            rpc.request(req).await?;
            rpc.is_ok()?;
        }
        AddonSubcommand::Configure(cmd) => match cmd {
            ConfigureAddonCommand::Okta {
                project_name,
                tenant,
                certificate,
                certificate_path,
                client_id,
                attributes,
            } => {
                let certificate = match (certificate, certificate_path) {
                    (Some(c), _) => c,
                    (_, Some(p)) => std::fs::read_to_string(p)?,
                    _ => unreachable!(),
                };

                let okta_config = OktaConfig::new(tenant, certificate, client_id, &attributes);
                let body = okta_config.clone();

                // Validate okta configuration
                let auth0 = Auth0Service::new(Auth0Provider::Okta(okta_config.into()));
                auth0.validate_provider_config().await?;

                // Do request
                let addon_id = "okta";
                let endpoint = format!("{}/{}", base_endpoint(&project_name)?, addon_id);
                let req =
                    Request::put(endpoint).body(CloudRequestWrapper::new(body, controller_route));
                rpc.request(req).await?;
                rpc.is_ok()?;
            }
        },
    };
    delete_embedded_node(&opts.config, rpc.node_name()).await;
    Ok(())
}

impl Output for Addon<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "Addon:")?;
        write!(w, "\n  Id: {}", self.id)?;
        write!(w, "\n  Enabled: {}", self.enabled)?;
        write!(w, "\n  Description: {}", self.description)?;
        writeln!(w)?;
        Ok(w)
    }
}

impl Output for Vec<Addon<'_>> {
    fn output(&self) -> anyhow::Result<String> {
        if self.is_empty() {
            return Ok("No addons found".to_string());
        }
        let mut w = String::new();
        for (idx, a) in self.iter().enumerate() {
            write!(w, "\n{idx}:")?;
            write!(w, "\n  Id: {}", a.id)?;
            write!(w, "\n  Enabled: {}", a.enabled)?;
            write!(w, "\n  Description: {}", a.description)?;
            writeln!(w)?;
        }
        Ok(w)
    }
}
