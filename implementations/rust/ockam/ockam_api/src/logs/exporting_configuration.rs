use crate::address::get_free_address;
use crate::config::UrlVar;
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use crate::logs::ExportingEnabled;
use crate::{CliState, TcpInlet};
use ockam_core::env::{get_env_with_default, FromString};
use ockam_core::errcode::{Kind, Origin};
use ockam_node::Executor;
use std::env::current_exe;
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::Duration;
use url::Url;

/// The exporting configuration contains all the parameters needed to configure the OpenTelemetry tracing layer.
///
/// Note: since this is the configuration for OpenTelemetry, this struct addresses the configuration
/// of both spans _and log records_ sent to an OpenTelemetry collector.
///
/// The configuration for log messages printed in a file, or in the console, use the LoggingConfiguration.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportingConfiguration {
    /// If TracingEnabled::On then spans and log records are sent to an OpenTelemetry collector.
    /// Some parameters for exporting the
    enabled: ExportingEnabled,
    /// Maximum time for exporting a batch of spans
    span_export_timeout: Duration,
    /// Maximum time for exporting a batch of log records
    log_export_timeout: Duration,
    /// Maximum time to wait until sending the current batch of spans
    span_export_scheduled_delay: Duration,
    /// Maximum time to wait until sending the current batch of logs
    log_export_scheduled_delay: Duration,
    /// Url of the OpenTelemetry collector
    opentelemetry_endpoint: Url,
    /// True if the user is an Ockam developer
    /// This boolean is set on spans to distinguish internal usage for external usage
    is_ockam_developer: bool,
}

impl ExportingConfiguration {
    /// Return true if distributed tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled == ExportingEnabled::On
    }

    /// Return true if the current user is an Ockam developer as determined by the OCKAM_DEVELOPER environment variable
    pub fn is_ockam_developer(&self) -> bool {
        self.is_ockam_developer
    }

    /// Return the maximum time for exporting a batch of log records
    pub fn log_export_timeout(&self) -> Duration {
        self.log_export_timeout
    }

    /// Return the maximum time to wait until sending the current batch of logs
    pub fn log_export_scheduled_delay(&self) -> Duration {
        self.log_export_scheduled_delay
    }

    /// Return the maximum time for exporting a batch of spans
    pub fn span_export_timeout(&self) -> Duration {
        self.span_export_timeout
    }

    /// Return the maximum time to wait until sending the current batch of spans
    pub fn span_export_scheduled_delay(&self) -> Duration {
        self.span_export_scheduled_delay
    }

    /// Return the URL where to export spans and log records
    pub fn opentelemetry_endpoint(&self) -> Url {
        self.opentelemetry_endpoint.clone()
    }

    /// Create a tracing configuration for a user command running in the foreground.
    /// (meaning that the process will shut down once the command has been executed)
    pub fn foreground(quiet: bool) -> ockam_core::Result<ExportingConfiguration> {
        match opentelemetry_endpoint()? {
            None => ExportingConfiguration::off(),
            Some(endpoint) => Ok(ExportingConfiguration {
                enabled: exporting_enabled(
                    quiet,
                    &endpoint,
                    opentelemetry_endpoint_foreground_connection_timeout()?,
                )?,
                span_export_timeout: span_export_timeout()?,
                log_export_timeout: log_export_timeout()?,
                span_export_scheduled_delay: foreground_span_export_scheduled_delay()?,
                log_export_scheduled_delay: foreground_log_export_scheduled_delay()?,
                opentelemetry_endpoint: endpoint.url(),
                is_ockam_developer: is_ockam_developer()?,
            }),
        }
    }

    /// Create a tracing configuration for a background node
    pub fn background(quiet: bool) -> ockam_core::Result<ExportingConfiguration> {
        match opentelemetry_endpoint()? {
            None => ExportingConfiguration::off(),
            Some(endpoint) => Ok(ExportingConfiguration {
                enabled: exporting_enabled(
                    quiet,
                    &endpoint,
                    opentelemetry_endpoint_background_connection_timeout()?,
                )?,
                span_export_timeout: span_export_timeout()?,
                log_export_timeout: log_export_timeout()?,
                span_export_scheduled_delay: background_span_export_scheduled_delay()?,
                log_export_scheduled_delay: background_log_export_scheduled_delay()?,
                opentelemetry_endpoint: endpoint.url(),
                is_ockam_developer: is_ockam_developer()?,
            }),
        }
    }

    /// Create a a tracing configuration which is disabled
    pub fn off() -> ockam_core::Result<ExportingConfiguration> {
        Ok(ExportingConfiguration {
            enabled: ExportingEnabled::Off,
            span_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            log_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            span_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            log_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            opentelemetry_endpoint: Self::default_opentelemetry_endpoint()?,
            is_ockam_developer: is_ockam_developer()?,
        })
    }

    /// Return the default endpoint for exporting traces
    fn default_opentelemetry_endpoint() -> ockam_core::Result<Url> {
        Ok(UrlVar::from_string(DEFAULT_OPENTELEMETRY_ENDPOINT)?.url)
    }
}

impl Display for ExportingConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("tracing")
            .field("enabled", &self.enabled.to_string())
            .finish()
    }
}

/// This enum represents the 2 possible endpoints for exporting traces. Either via:
///
///  - An HTTPS collector
///  - An Ockam portal to an HTTPS collector
///
#[derive(Debug, Clone)]
pub enum OpenTelemetryEndpoint {
    PortalEndpoint(Url),
    HttpsEndpoint(Url),
}

impl OpenTelemetryEndpoint {
    /// Return the URL to connect to
    pub fn url(&self) -> Url {
        match self {
            OpenTelemetryEndpoint::PortalEndpoint(url) => url.clone(),
            OpenTelemetryEndpoint::HttpsEndpoint(url) => url.clone(),
        }
    }
}

/// Return true if tracing is enabled, as decided by the OCKAM_TRACING environment variable.
///
/// FOR NOW THE DEFAULT IS FALSE.
/// TODO: set it to true after enough testing!
///
pub fn is_exporting_set() -> ockam_core::Result<bool> {
    get_env_with_default(OCKAM_OPENTELEMETRY_EXPORT, false)
}

/// Return ExportingEnabled::On if:
///
/// - Exporting has not been deactivated by the user
/// - The opentelemetry endpoint is accessible
///
fn exporting_enabled(
    quiet: bool,
    endpoint: &OpenTelemetryEndpoint,
    connection_check_timeout: Duration,
) -> ockam_core::Result<ExportingEnabled> {
    if is_endpoint_accessible(&endpoint.url(), connection_check_timeout) {
        Ok(ExportingEnabled::On)
    } else {
        if !quiet {
            let endpoint_kind = match endpoint {
                OpenTelemetryEndpoint::HttpsEndpoint(_) => "OpenTelemetry collector endpoint",
                OpenTelemetryEndpoint::PortalEndpoint(_) => "opentelemetry inlet",
            };
            println!("Exporting OpenTelemetry events is disabled because the {} at {} cannot be reached after {}ms", endpoint_kind, endpoint.url(), connection_check_timeout.as_millis());
            println!("You can disable the export of OpenTelemetry events with: `export OCKAM_OPENTELEMETRY_EXPORT=false` to avoid this connection check.");
        }
        Ok(ExportingEnabled::Off)
    }
}

/// Return true if the endpoint can be accessed with a TCP connection
fn is_endpoint_accessible(url: &Url, connection_check_timeout: Duration) -> bool {
    match to_socket_addr(url) {
        Some(address) => {
            std::net::TcpStream::connect_timeout(&address, connection_check_timeout).is_ok()
        }
        _ => false,
    }
}

/// Return a SocketAddr corresponding to the Url
fn to_socket_addr(url: &Url) -> Option<SocketAddr> {
    match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => (host, port)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next()),
        // the port might be unspecified, in that case we use 443, a HTTPS port
        (Some(host), None) => (host, 443)
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next()),
        _ => None,
    }
}

/// Return the tracing endpoint, defined by an environment variable
/// If the endpoint can be established with an Ockam portal to the opentelemetry-relay created in the project
/// use that URL, otherwise use the HTTPS endpoint
fn opentelemetry_endpoint() -> ockam_core::Result<Option<OpenTelemetryEndpoint>> {
    if !is_exporting_set()? {
        Ok(None)
    } else {
        let cli_state = CliState::with_default_dir()?;
        if let Ok(Ok(url)) = Executor::execute_future(async move {
            // if a project is defined try to use the OpenTelemetry portal
            if cli_state.projects().get_default_project().await.is_ok() {
                get_project_endpoint_url(&cli_state).await
            } else {
                get_https_endpoint()
            }
        }) {
            Ok(Some(url))
        } else {
            // no endpoint could be set locally
            Ok(None)
        }
    }
}

/// When a project exists, return the OpenTelemetry node inlet address
/// If the node does not exist, create it.
/// If the node is not running, restart it.
async fn get_project_endpoint_url(
    cli_state: &CliState,
) -> ockam_core::Result<OpenTelemetryEndpoint> {
    let node = cli_state.get_node(OCKAM_OPENTELEMETRY_NODE_NAME).await.ok();
    match node {
        // If no opentelemetry node is running locally, start one
        None => {
            let url = start_opentelemetry_node().await?;
            Ok::<OpenTelemetryEndpoint, ockam_core::Error>(OpenTelemetryEndpoint::PortalEndpoint(
                url,
            ))
        }
        Some(node) => {
            // If a node exists and is running, use it
            if node.is_running() {
                let tcp_inlet = get_opentelemetry_inlet(&cli_state).await?;
                let url = socket_addr_to_url(&tcp_inlet.bind_addr().to_string())?;
                Ok(OpenTelemetryEndpoint::PortalEndpoint(url))
            } else {
                // if the node is not running, restart it and recreate the inlet
                let url = restart_opentelemetry_node().await?;
                Ok(OpenTelemetryEndpoint::PortalEndpoint(url))
            }
        }
    }
}

/// Return the inlet used to export OpenTelemetry traces
async fn get_opentelemetry_inlet(cli_state: &CliState) -> ockam_core::Result<TcpInlet> {
    Ok(cli_state
        .get_tcp_inlet(
            OCKAM_OPENTELEMETRY_NODE_NAME,
            OCKAM_OPENTELEMETRY_INLET_ALIAS,
        )
        .await?)
}

/// Return the default HTTPs endpoint
fn get_https_endpoint() -> ockam_core::Result<OpenTelemetryEndpoint> {
    Ok(OpenTelemetryEndpoint::HttpsEndpoint(
        get_env_with_default(
            OCKAM_OPENTELEMETRY_ENDPOINT,
            UrlVar::new(ExportingConfiguration::default_opentelemetry_endpoint()?),
        )?
        .url,
    ))
}

/// Return true if the current user is an internal user
fn is_ockam_developer() -> ockam_core::Result<bool> {
    get_env_with_default(OCKAM_DEVELOPER, false)
}

/// Return the export timeout for spans, defined by an environment variable
pub fn span_export_timeout() -> ockam_core::Result<Duration> {
    get_env_with_default(OCKAM_SPAN_EXPORT_TIMEOUT, DEFAULT_EXPORT_TIMEOUT)
}

/// Return the endpoint connection timeout, for a background node, defined by an environment variable
fn opentelemetry_endpoint_background_connection_timeout() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_BACKGROUND_OPENTELEMETRY_ENDPOINT_CONNECTION_TIMEOUT,
        DEFAULT_OPENTELEMETRY_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT,
    )
}

/// Return the endpoint connection timeout, for a foreground command, defined by an environment variable
fn opentelemetry_endpoint_foreground_connection_timeout() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_FOREGROUND_OPENTELEMETRY_ENDPOINT_CONNECTION_TIMEOUT,
        DEFAULT_OPENTELEMETRY_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT,
    )
}

/// Return the delay between the export of 2 spans batches, for a foreground command, defined by an environment variable
fn foreground_span_export_scheduled_delay() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY,
        DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
    )
}

/// Return the delay between the export of 2 spans batches, for a background node, defined by an environment variable
fn background_span_export_scheduled_delay() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_BACKGROUND_SPAN_EXPORT_SCHEDULED_DELAY,
        DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY,
    )
}

/// Return the export timeout for log records, defined by an environment variable
pub fn log_export_timeout() -> ockam_core::Result<Duration> {
    get_env_with_default(OCKAM_LOG_EXPORT_TIMEOUT, DEFAULT_EXPORT_TIMEOUT)
}

/// Return the delay between the export of 2 logs batches, for a foreground command, defined by an environment variable
pub fn foreground_log_export_scheduled_delay() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_FOREGROUND_LOG_EXPORT_SCHEDULED_DELAY,
        DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
    )
}

/// Return the delay between the export of 2 logs batches, for a background node, defined by an environment variable
pub fn background_log_export_scheduled_delay() -> ockam_core::Result<Duration> {
    get_env_with_default(
        OCKAM_BACKGROUND_LOG_EXPORT_SCHEDULED_DELAY,
        DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY,
    )
}

/// Delete the opentelemetry node and recreate it to restart the inlet
async fn restart_opentelemetry_node() -> ockam_core::Result<Url> {
    let args = vec![
        "node".to_string(),
        "delete".to_string(),
        "-y".to_string(),
        OCKAM_OPENTELEMETRY_NODE_NAME.to_string(),
    ];
    run_ockam(args).await?;
    start_opentelemetry_node().await
}

/// Start a node with an that will forward traces via a relay deployed by the authority node.
/// The relay is connected to an outlet which sends traces to the OpenTelemetry collector
async fn start_opentelemetry_node() -> ockam_core::Result<Url> {
    // get a free address for the inlet
    let local_address = get_free_address().map_err(|e| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Io,
            format!("cannot get a free address on this machine: {e:?}"),
        )
    })?;
    // configure a node with an
    let config = format!(
        "{{nodes: {OCKAM_OPENTELEMETRY_NODE_NAME}, tcp-inlets: {{opentelemetry-inlet: {{at: {OCKAM_OPENTELEMETRY_NODE_NAME}, from: '{local_address}', via: {OCKAM_OPENTELEMETRY_RELAY_NAME}, alias: opentelemetry-inlet}}}}}}"
    );
    let args = vec![
        "run".to_string(),
        "--inline".to_string(),
        config.to_string(),
    ];
    run_ockam(args).await?;
    socket_addr_to_url(&local_address.to_string())
}

/// Create a URL from a socket address
fn socket_addr_to_url(socket_addr: &str) -> ockam_core::Result<Url> {
    Ok(
        Url::from_str(&format!("http://{socket_addr}")).map_err(|e| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("{} is not a valid URL: {e:?}", socket_addr),
            )
        })?,
    )
}

/// Run the ockam command line with specific arguments
async fn run_ockam(args: Vec<String>) -> ockam_core::Result<()> {
    let ockam_exe = current_exe().map_err(|e| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Io,
            format!("cannot get the current ockam exe: {e:?}"),
        )
    })?;

    Command::new(ockam_exe)
        .args(args.clone())
        .env(OCKAM_OPENTELEMETRY_EXPORT, "false")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| {
            println!(
                "{}",
                format!(
                    "cannot start the ockam command with arguments: {:?}. Got: {e:?}",
                    args.join(",")
                )
            );
            ockam_core::Error::new(
                Origin::Api,
                Kind::Io,
                format!(
                    "cannot run the ockam command with arguments: {:?}. Got: {e:?}",
                    args.join(",")
                ),
            )
        })?;
    Ok(())
}
