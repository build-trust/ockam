use core::fmt::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use reqwest::Url;
use rustls::{Certificate, ClientConfig, ClientConnection, Connection, RootCertStore, Stream};

use ockam::Context;
use ockam_api::cloud::addon::Addon;
use ockam_api::cloud::project::{OktaConfig, Project};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::delete_embedded_node;
use crate::project::config;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{api, node_rpc, Rpc};
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
            default_value = "default",
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
        #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
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
        AddonSubcommand::Configure(scmd) => match scmd {
            ConfigureAddonCommand::Okta {
                project_name,
                tenant,
                certificate,
                certificate_path,
                client_id,
                attributes,
            } => {
                let base_url = Url::parse(tenant.as_str()).context("could not parse tenant url")?;
                let domain = base_url
                    .host_str()
                    .context("could not read domain from tenant url")?;

                let certificate = match (certificate, certificate_path) {
                    (Some(c), _) => c,
                    (_, Some(p)) => std::fs::read_to_string(p)?,
                    _ => query_certificate_chain(domain)?,
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
                println!("Okta addon enabled");

                // Wait until project is ready again
                println!("Getting things ready for project...");
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                let project_id = config::get_project(&opts.config, &project_name)
                    .context("project not found in lookup")?;
                rpc.request(api::project::show(&project_id, controller_route))
                    .await?;
                let project: Project = rpc.parse_response()?;
                check_project_readiness(
                    &ctx,
                    &opts,
                    &cmd.cloud_opts,
                    rpc.node_name(),
                    None,
                    project,
                )
                .await?;

                println!("Okta addon configured successfully");
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

pub fn query_certificate_chain(domain: &str) -> anyhow::Result<String> {
    use std::io::Write;
    let domain_with_port = domain.to_string() + ":443";

    // Setup Root Certificate Store
    let mut root_certificate_store = RootCertStore::empty();
    for c in rustls_native_certs::load_native_certs()? {
        root_certificate_store.add(&Certificate(c.0))?;
    }

    // Configure TLS Client
    let client_configuration = Arc::new(
        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_certificate_store)
            .with_no_client_auth(),
    );

    // Make an HTTP request
    let mut client_connection = ClientConnection::new(client_configuration, domain.try_into()?)?;
    let mut tcp_stream = TcpStream::connect(domain_with_port)?;
    let mut stream = Stream::new(&mut client_connection, &mut tcp_stream);
    stream
        .write_all(
            format!(
                "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n",
                domain
            )
                .as_bytes(),
        )
        .context("failed to write to tcp stream")?;

    let connection = Connection::try_from(client_connection)?;
    let certificate_chain = connection
        .peer_certificates()
        .context("could not discover certificate chain")?;

    // Encode a PEM encoded certificate chain
    let label = "CERTIFICATE";
    let mut encoded = String::new();
    for certificate in certificate_chain {
        let bytes = certificate.0.clone();
        let pem = pem_rfc7468::encode_string(label, pem_rfc7468::LineEnding::LF, &bytes)
            .context("could not encode certificate to PEM")?;
        encoded = encoded + &pem;
    }

    Ok(encoded)
}
