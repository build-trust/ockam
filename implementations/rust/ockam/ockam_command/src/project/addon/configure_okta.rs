use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;

use clap::builder::NonEmptyStringValueParser;
use clap::Args;
use colorful::Colorful;
use miette::{miette, Context as _, IntoDiagnostic};
use rustls::{ClientConfig, ClientConnection, Connection, RootCertStore, Stream};
use rustls_pki_types::ServerName;

use ockam::Context;
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::enroll::okta_oidc_provider::OktaOidcProvider;
use ockam_api::fmt_ok;
use ockam_api::minicbor_url::Url;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::addon::Addons;
use ockam_api::orchestrator::project::models::OktaConfig;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::project::addon::check_configuration_completion;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/configure_influxdb/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/configure_influxdb/after_long_help.txt");

/// Configure the Okta addon for a project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct AddonConfigureOktaSubcommand {
    #[arg(
        help = docs::about("Ockam Project name"),
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

    /// Okta Client ID
    #[arg(
        long,
        id = "client_id",
        value_name = "CLIENT_ID",
        value_parser(NonEmptyStringValueParser::new())
    )]
    client_id: String,

    #[arg(help = docs::about("Attributes names to copy from Okta userprofile into Ockam credential"))]
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,
}

impl AddonConfigureOktaSubcommand {
    pub fn name(&self) -> String {
        "project addon configure okta".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let project_id = opts
            .state
            .projects()
            .get_project_by_name(&self.project_name)
            .await?
            .project_id()
            .to_string();

        let base_url = Url::parse(self.tenant.as_str())
            .into_diagnostic()
            .context("could not parse tenant url")?;
        let domain = base_url
            .host_str()
            .ok_or(miette!("could not read domain from tenant url"))?;

        let certificate = match (&self.certificate, &self.certificate_path) {
            (Some(c), _) => c.to_string(),
            (_, Some(p)) => std::fs::read_to_string(p).into_diagnostic()?,
            _ => query_certificate_chain(domain)?,
        };

        let okta_config = OktaConfig::new(
            base_url,
            certificate,
            self.client_id.clone(),
            self.attributes.clone(),
        );

        // Validate okta configuration
        let auth0 = OidcService::new_with_provider(Arc::new(OktaOidcProvider::new(
            okta_config.clone().into(),
        )));
        auth0.validate_provider_config().await?;

        // Do request
        let node = InMemoryNode::start(ctx, &opts.state).await?;
        let controller = node.create_controller().await?;

        let response = controller
            .configure_okta_addon(ctx, &project_id, okta_config)
            .await?;
        check_configuration_completion(&opts, ctx, &node, &project_id, &response.operation_id)
            .await?;

        opts.terminal
            .write_line(fmt_ok!("Okta addon configured successfully"))?;

        Ok(())
    }
}

fn query_certificate_chain(domain: &str) -> Result<String> {
    use std::io::Write;
    let domain_with_port = domain.to_string() + ":443";

    // Setup Root Certificate Store
    let mut root_certificate_store = RootCertStore::empty();

    let certificates = rustls_native_certs::load_native_certs();
    if let Some(e) = certificates.errors.first() {
        Err(Error::new(
            Origin::Transport,
            Kind::Io,
            format!("Cannot load the native certificates: {e:?}"),
        ))?
    };

    let certificates = certificates.certs;

    for c in certificates {
        root_certificate_store
            .add(c)
            .into_diagnostic()
            .wrap_err("failed to add certificate to root certificate store")?;
    }

    // Configure TLS Client
    let client_configuration = Arc::new(
        ClientConfig::builder()
            .with_root_certificates(root_certificate_store)
            .with_no_client_auth(),
    );

    // Make an HTTP request
    let server_name: ServerName = domain
        .try_into()
        .into_diagnostic()
        .wrap_err("failed to convert domain to a ServerName")?;
    let mut client_connection = ClientConnection::new(client_configuration, server_name.to_owned())
        .into_diagnostic()
        .wrap_err("failed to create a client connection")?;
    let mut tcp_stream = TcpStream::connect(domain_with_port).into_diagnostic()?;
    let mut stream = Stream::new(&mut client_connection, &mut tcp_stream);
    stream
        .write_all(
            format!(
                "GET / HTTP/1.1\r\nHost: {domain}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n"
            )
                .as_bytes(),
        )
        .into_diagnostic().wrap_err("failed to write to tcp stream")?;

    let connection = Connection::from(client_connection);
    let certificate_chain = connection
        .peer_certificates()
        .ok_or(miette!("could not discover certificate chain"))?;

    // Encode a PEM encoded certificate chain
    let label = "CERTIFICATE";
    let mut encoded = String::new();
    for certificate in certificate_chain {
        let bytes = certificate.as_ref();
        let pem = pem_rfc7468::encode_string(label, pem_rfc7468::LineEnding::LF, bytes)
            .into_diagnostic()
            .wrap_err("could not encode certificate to PEM")?;
        encoded = encoded + &pem;
    }

    Ok(encoded)
}
