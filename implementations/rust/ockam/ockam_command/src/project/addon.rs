use core::fmt::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context as _};
use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Subcommand};
use reqwest::Url;
use rustls::{Certificate, ClientConfig, ClientConnection, Connection, RootCertStore, Stream};
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;

use ockam::Context;
use ockam_api::cloud::addon::{Addon, ConfluentConfig};
use ockam_api::cloud::project::{InfluxDBTokenLeaseManagerConfig, OktaConfig, Project};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_core::api::Request;
use ockam_core::CowStr;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::delete_embedded_node;
use crate::project::config;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{api, exitcode, node_rpc, Rpc};
use crate::{help, CommandGlobalOpts, Result};

const INFLUXDB_HELP_DETAIL: &str = r#"
About:
    InfluxDB addon allows you to create, store and retrieve InfluxDB Tokens with expiry times.

Examples:
    Examples of how to configure and use the InfluxDB Cloud addon can be found within the example documentation.
    https://docs.ockam.io/guides/examples/influxdb-cloud-token-lease-management
"#;

const CONFLUENT_HELP_DETAIL: &str = r#"
About:
    Confluent Cloud addon allows you to enable end-to-end encryption with your Kafka Consumers and Kafka Producers

Examples:
    Examples of how to configure and use the Confluent Cloud addon can be found within the example documentation.
    https://docs.ockam.io/guides/examples/end-to-end-encrypted-kafka
"#;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, subcommand_required = true)]
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
    /// Configure the Okta addon for a project
    Okta {
        /// Ockam Project name
        #[arg(
            long = "project",
            id = "project",
            value_name = "PROJECT_NAME",
            default_value = "default",
            value_parser(NonEmptyStringValueParser::new())
        )]
        project_name: String,

        /// Okta Plugin tenant URL
        #[arg(
            long,
            id = "tenant",
            value_name = "TENANT",
            value_parser(NonEmptyStringValueParser::new())
        )]
        tenant: String,

        /// Okta Certificate. Use either this or --cert-path
        #[arg(
            long = "cert",
            group = "cert",
            value_name = "CERTIFICATE",
            value_parser(NonEmptyStringValueParser::new())
        )]
        certificate: Option<String>,

        /// Okta Certificate file path. Use either this or --cert
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
    /// Configure the InfluxDB Cloud addon for a project
    #[command(after_long_help = help::template(INFLUXDB_HELP_DETAIL))]
    InfluxDb {
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
    },
    /// Configure the Confluent Cloud addon for a project
    #[command(after_long_help = help::template(CONFLUENT_HELP_DETAIL))]
    Confluent {
        /// Ockam project name
        #[arg(
            long = "project",
            id = "project",
            value_name = "PROJECT_NAME",
            default_value = "default",
            value_parser(NonEmptyStringValueParser::new())
        )]
        project_name: String,

        /// Confluent Cloud bootstrap server address
        #[arg(
            long,
            id = "bootstrap_server",
            value_name = "BOOTSTRAP_SERVER",
            value_parser(NonEmptyStringValueParser::new())
        )]
        bootstrap_server: String,
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
                "Failed to get project {project_name} from config lookup"
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
        AddonSubcommand::Configure(scmd) => {
            match scmd {
                ConfigureAddonCommand::Okta {
                    project_name,
                    tenant,
                    certificate,
                    certificate_path,
                    client_id,
                    attributes,
                } => {
                    let base_url =
                        Url::parse(tenant.as_str()).context("could not parse tenant url")?;
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
                    let req = Request::put(endpoint).body(CloudRequestWrapper::new(
                        body,
                        controller_route,
                        None::<CowStr>,
                    ));
                    rpc.request(req).await?;
                    rpc.is_ok()?;
                    println!("Okta addon enabled");

                    // Wait until project is ready again
                    println!("Getting things ready for project...");
                    tokio::time::sleep(std::time::Duration::from_secs(45)).await;
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
                ConfigureAddonCommand::InfluxDb {
                    project_name,
                    endpoint_url,
                    token,
                    org_id,
                    permissions,
                    permissions_path,
                    max_ttl_secs,
                    user_access_role,
                    admin_access_role,
                } => {
                    let perms = match (permissions, permissions_path) {
                        (_, Some(p)) => std::fs::read_to_string(p)?,
                        (Some(perms), _) => perms,
                        _ => {
                            return Err(crate::error::Error::new(exitcode::IOERR, anyhow!("Permissions JSON is required, supply --permissions or --permissions-path.")));
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
                    let endpoint = format!("{}/{}", base_endpoint(&project_name)?, add_on_id);
                    let req = Request::put(endpoint).body(CloudRequestWrapper::new(
                        body,
                        controller_route,
                        None::<CowStr>,
                    ));

                    rpc.request(req).await?;
                    rpc.is_ok()?;
                    println!("InfluxDB addon enabled");

                    // Wait until project is ready again
                    println!("Getting things ready for project...");
                    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

                    let project_id = config::get_project(&opts.config, &project_name)
                        .context("project not found in lookup")?;

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

                        check_project_readiness(
                            &ctx,
                            &opts,
                            &cmd.cloud_opts,
                            rpc.node_name(),
                            None,
                            project,
                        )
                        .await?;
                    }
                    println!("InfluxDB addon configured successfully");
                }
                ConfigureAddonCommand::Confluent {
                    project_name,
                    bootstrap_server,
                } => {
                    let body = ConfluentConfig::new(bootstrap_server);
                    let addon_id = "confluent";
                    let endpoint = format!("{}/{}", base_endpoint(&project_name)?, addon_id);
                    let req = Request::put(endpoint).body(CloudRequestWrapper::new(
                        body,
                        controller_route,
                        None::<CowStr>,
                    ));
                    rpc.request(req).await?;
                    rpc.is_ok()?;
                    println!("Confluent addon enabled");

                    // Wait until project is ready again
                    println!("Getting things ready for project...");
                    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

                    let project_id = config::get_project(&opts.config, &project_name)
                        .context("project not found in lookup")?;
                    let retry_strategy = FixedInterval::from_millis(5000).take(4);
                    Retry::spawn(retry_strategy, || async {
                        let mut rpc = rpc.clone();
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
                        Ok(())
                    })
                    .await
                    .map_err(|e: crate::error::Error| anyhow!(e.to_string()))?;
                    println!("Confluent addon configured successfully");
                }
            }
        }
    };
    delete_embedded_node(&opts, rpc.node_name()).await;
    Ok(())
}

impl Output for Addon<'_> {
    fn output(&self) -> Result<String> {
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
    fn output(&self) -> Result<String> {
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

pub fn query_certificate_chain(domain: &str) -> Result<String> {
    use std::io::Write;
    let domain_with_port = domain.to_string() + ":443";

    // Setup Root Certificate Store
    let mut root_certificate_store = RootCertStore::empty();
    for c in rustls_native_certs::load_native_certs()? {
        root_certificate_store
            .add(&Certificate(c.0))
            .context("failed to add certificate to root certificate store")?;
    }

    // Configure TLS Client
    let client_configuration = Arc::new(
        ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_certificate_store)
            .with_no_client_auth(),
    );

    // Make an HTTP request
    let server_name = domain
        .try_into()
        .context("failed to convert domain to a ServerName")?;
    let mut client_connection = ClientConnection::new(client_configuration, server_name)
        .context("failed to create a client connection")?;
    let mut tcp_stream = TcpStream::connect(domain_with_port)?;
    let mut stream = Stream::new(&mut client_connection, &mut tcp_stream);
    stream
        .write_all(
            format!(
                "GET / HTTP/1.1\r\nHost: {domain}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n"
            )
                .as_bytes(),
        )
        .context("failed to write to tcp stream")?;

    let connection = Connection::try_from(client_connection).context("failed to get connection")?;
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
