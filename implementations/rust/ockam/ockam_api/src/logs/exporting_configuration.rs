use crate::cli_state::DEFAULT_NODE_NAME;
use crate::config::UrlVar;
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use crate::logs::ExportingEnabled;
use crate::{ApiError, CliState, TransportRouteResolver};
use crate::{DefaultAddress, Result};
use ockam::identity::{get_default_timeout, Identifier, SecureClient, TrustIdentifierPolicy};
use ockam_core::env::{get_env_with_default, FromString};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use std::fmt::{Debug, Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_retry::strategy::FibonacciBackoff;
use url::Url;

/// The exporting configuration contains all the parameters needed to configure the OpenTelemetry tracing layer.
///
/// Note: since this is the configuration for OpenTelemetry, this struct addresses the configuration
/// of both spans _and log records_ sent to an OpenTelemetry collector.
///
/// The configuration for log messages printed in a file, or in the console, use the LoggingConfiguration.
///
/// When a portal is used to export log records and traces we don't wait for a response from the
/// OpenTelemetry collector because this makes the command line less responsive.
///
/// The time necessary to send just a batch can be configured with the 'cutoff' variables.
///
#[derive(Debug, Clone)]
pub struct ExportingConfiguration {
    /// If TracingEnabled::On then spans and log records are sent to an OpenTelemetry collector.
    /// Some parameters for exporting the
    enabled: ExportingEnabled,
    /// Maximum time for exporting a batch of spans (with a response)
    span_export_timeout: Duration,
    /// Maximum time for exporting a batch of log records (with a response)
    log_export_timeout: Duration,
    /// Maximum time to wait until sending the current batch of spans
    span_export_scheduled_delay: Duration,
    /// Maximum time to wait until sending the current batch of logs
    log_export_scheduled_delay: Duration,
    /// Size of the queue used to batch spans
    span_export_queue_size: u16,
    /// Size of the queue used to batch logs
    log_export_queue_size: u16,
    /// Endpoint for the telemetry collector
    opentelemetry_endpoint: TelemetryEndpoint,
    /// True if the user is an Ockam developer
    /// This boolean is set on spans to distinguish internal usage for external usage
    is_ockam_developer: bool,
    /// Maximum time for exporting a batch of spans (with no response)
    span_export_cutoff: Option<Duration>,
    /// Maximum time for exporting a batch of log records (with no response)
    log_export_cutoff: Option<Duration>,
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

    /// Size of the queue used for batching spans
    pub fn span_export_queue_size(&self) -> u16 {
        self.span_export_queue_size
    }

    /// Size of the queue used for batching log records
    pub fn log_export_queue_size(&self) -> u16 {
        self.log_export_queue_size
    }

    /// Return the maximum time to wait until sending the current batch of spans (without waiting for a response)
    pub fn span_export_cutoff(&self) -> Option<Duration> {
        self.span_export_cutoff
    }

    /// Return the maximum time to wait until sending the current batch of log records (without waiting for a response)
    pub fn log_export_cutoff(&self) -> Option<Duration> {
        self.log_export_cutoff
    }

    /// Return the URL where to export spans and log records
    pub fn opentelemetry_endpoint(&self) -> TelemetryEndpoint {
        self.opentelemetry_endpoint.clone()
    }

    /// Create a tracing configuration for a user command running in the foreground.
    /// (meaning that the process will shut down once the command has been executed)
    pub async fn foreground(state: &CliState, ctx: &Context) -> Result<ExportingConfiguration> {
        match opentelemetry_endpoint(state, ctx).await? {
            None => ExportingConfiguration::off(),
            Some(endpoint) => {
                let enabled = exporting_enabled(
                    &endpoint,
                    ctx,
                    opentelemetry_endpoint_foreground_connection_timeout()?,
                )
                .await?;
                Self::make_foreground_exporting_configuration(endpoint, enabled)
            }
        }
    }

    /// Create a tracing configuration for a background node
    pub async fn background(state: &CliState, ctx: &Context) -> Result<ExportingConfiguration> {
        match opentelemetry_endpoint(state, ctx).await? {
            None => ExportingConfiguration::off(),
            Some(endpoint) => {
                let enabled = exporting_enabled(
                    &endpoint,
                    ctx,
                    opentelemetry_endpoint_background_connection_timeout()?,
                )
                .await?;
                Self::make_background_exporting_configuration(endpoint, enabled)
            }
        }
    }

    /// Create a a tracing configuration which is disabled
    pub fn off() -> Result<ExportingConfiguration> {
        Ok(ExportingConfiguration {
            enabled: ExportingEnabled::Off,
            span_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            log_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            span_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            log_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            span_export_queue_size: DEFAULT_SPAN_EXPORT_QUEUE_SIZE,
            log_export_queue_size: DEFAULT_LOG_EXPORT_QUEUE_SIZE,
            opentelemetry_endpoint: Self::default_telemetry_endpoint()?,
            is_ockam_developer: is_ockam_developer()?,
            span_export_cutoff: None,
            log_export_cutoff: None,
        })
    }

    pub fn make_foreground_exporting_configuration(
        endpoint: TelemetryEndpoint,
        enabled: ExportingEnabled,
    ) -> Result<ExportingConfiguration> {
        Ok(ExportingConfiguration {
            enabled,
            span_export_timeout: span_export_timeout()?,
            log_export_timeout: log_export_timeout()?,
            span_export_scheduled_delay: foreground_span_export_scheduled_delay()?,
            log_export_scheduled_delay: foreground_log_export_scheduled_delay()?,
            span_export_queue_size: span_export_queue_size()?,
            log_export_queue_size: log_export_queue_size()?,
            opentelemetry_endpoint: endpoint,
            is_ockam_developer: is_ockam_developer()?,
            span_export_cutoff: Some(foreground_span_export_portal_cutoff()?),
            log_export_cutoff: Some(foreground_log_export_cutoff()?),
        })
    }

    pub fn make_background_exporting_configuration(
        endpoint: TelemetryEndpoint,
        enabled: ExportingEnabled,
    ) -> Result<ExportingConfiguration> {
        Ok(ExportingConfiguration {
            enabled,
            span_export_timeout: span_export_timeout()?,
            log_export_timeout: log_export_timeout()?,
            span_export_scheduled_delay: background_span_export_scheduled_delay()?,
            log_export_scheduled_delay: background_log_export_scheduled_delay()?,
            span_export_queue_size: span_export_queue_size()?,
            log_export_queue_size: log_export_queue_size()?,
            opentelemetry_endpoint: endpoint,
            is_ockam_developer: is_ockam_developer()?,
            span_export_cutoff: Some(background_span_export_portal_cutoff()?),
            log_export_cutoff: Some(background_log_export_cutoff()?),
        })
    }

    /// Return the default endpoint for exporting traces
    fn default_telemetry_endpoint() -> Result<TelemetryEndpoint> {
        get_https_endpoint()
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
///  - An gRPC forwarder on a remote node, accessed via a secure channel
///
#[derive(Clone)]
pub enum TelemetryEndpoint {
    SecureChannelEndpoint(SecureClient, String),
    HttpsEndpoint(Url),
}

impl Display for TelemetryEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryEndpoint::SecureChannelEndpoint(client, forwarder_service_name) => f
                .write_fmt(format_args!(
                    "{} => 0#{}",
                    &client.secure_route().to_string(),
                    forwarder_service_name
                )),
            TelemetryEndpoint::HttpsEndpoint(url) => f.write_str(url.as_str()),
        }
    }
}

impl Debug for TelemetryEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

/// Return true if the export of traces and log records is enabled,
/// as decided by the OCKAM_TELEMETRY_EXPORT environment variable.
pub fn is_exporting_set() -> Result<bool> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT,
        // fallback to legacy env var
        get_env_with_default(OCKAM_OPENTELEMETRY_EXPORT, true)?,
    )?)
}

/// Return true if traces and log records can be exported via a portal (when a project exists),
/// as decided by the OCKAM_TELEMETRY_EXPORT_VIA_PROJECT environment variable.
pub fn is_exporting_via_project_set() -> Result<bool> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT_VIA_PROJECT,
        true,
    )?)
}

/// Return true if traces and log records can be exported via a portal (when a project exists),
/// as decided by the OCKAM_TELEMETRY_EXPORT_VIA_PROJECT environment variable.
pub fn is_exporting_via_authority_set() -> Result<bool> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT_VIA_AUTHORITY,
        false,
    )?)
}

/// Return the route to the node accepting telemetry data
/// as decided by the OCKAM_TELEMETRY_EXPORT_NODE_ROUTE environment variable.
pub fn telemetry_export_node_route() -> Result<Option<MultiAddr>> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT_NODE_ROUTE,
        None,
    )?)
}

/// Return the identifier to the node accepting telemetry data
/// as decided by the OCKAM_TELEMETRY_EXPORT_NODE_IDENTIFIER environment variable.
pub fn telemetry_export_node_identifier() -> Result<Option<Identifier>> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT_NODE_IDENTIFIER,
        None,
    )?)
}

/// Return the name of the service collecting telemetry data on a remote node
/// as decided by the OCKAM_TELEMETRY_EXPORT_NODE_FORWARDER_SERVICE environment variable.
pub fn telemetry_export_node_forwarder_service() -> Result<String> {
    Ok(get_env_with_default(
        OCKAM_TELEMETRY_EXPORT_NODE_FORWARDER_SERVICE,
        DefaultAddress::GRPC_FORWARDER.to_string(),
    )?)
}

/// Return true to display messages during the setup of the export
pub fn is_export_debug_set() -> Result<bool> {
    Ok(get_env_with_default(
        OCKAM_OPENTELEMETRY_EXPORT_DEBUG,
        false,
    )?)
}

/// Print a debug statement if OCKAM_OPENTELEMETRY_EXPORT_DEBUG is true
fn print_debug(message: impl Into<String>) {
    if is_export_debug_set().unwrap_or(false) {
        println!("{}", message.into());
    }
}

/// Return ExportingEnabled::On if:
///
/// - Exporting has not been deactivated by the user
/// - The telemetry endpoint is accessible
///
async fn exporting_enabled(
    endpoint: &TelemetryEndpoint,
    ctx: &Context,
    connection_check_timeout: Duration,
) -> ockam_core::Result<ExportingEnabled> {
    if is_endpoint_accessible(endpoint, ctx, connection_check_timeout).await {
        print_debug("Exporting is enabled");
        Ok(ExportingEnabled::On)
    } else {
        let endpoint_kind = match endpoint {
            TelemetryEndpoint::HttpsEndpoint(_) => "HTTPs telemetry collector endpoint",
            TelemetryEndpoint::SecureChannelEndpoint(_, _) => "Node telemetry collector endpoint",
        };
        print_debug(format!("Exporting telemetry events is disabled because the {} at {} cannot be reached after {}ms", endpoint_kind, endpoint, connection_check_timeout.as_millis()));
        print_debug("You can disable the export of telemetry events with: `export OCKAM_TELEMETRY_EXPORT=false` to avoid this connection check.");

        print_debug("Exporting is disabled");
        Ok(ExportingEnabled::Off)
    }
}

/// Return true if the endpoint can be accessed with a TCP connection
async fn is_endpoint_accessible(
    endpoint: &TelemetryEndpoint,
    ctx: &Context,
    connection_check_timeout: Duration,
) -> bool {
    match endpoint {
        TelemetryEndpoint::SecureChannelEndpoint(client, _) => {
            is_node_accessible(client, ctx, &connection_check_timeout).await
        }
        TelemetryEndpoint::HttpsEndpoint(url) => {
            print_debug("check if the endpoint is accessible");
            is_url_accessible(url, connection_check_timeout).await
        }
    }
}

/// Return true if the project node can be accessed with a secure channel
async fn is_node_accessible(
    secure_client: &SecureClient,
    ctx: &Context,
    connection_check_timeout: &Duration,
) -> bool {
    secure_client
        .clone()
        .with_secure_channel_timeout(connection_check_timeout)
        .check_secure_channel(ctx)
        .await
        .is_ok()
}

/// Return true if the URL can be accessed with a TCP connection
async fn is_url_accessible(url: &Url, connection_check_timeout: Duration) -> bool {
    match to_socket_addr(url) {
        Some(address) => {
            let retries = FibonacciBackoff::from_millis(100);
            let now = Instant::now();

            // TODO: Not sure we need to retry really, also maybe it could happen in the background
            //  to not slow things down
            for timeout_duration in retries {
                print_debug(format!(
                    "trying to connect to {address} in {timeout_duration:?}"
                ));

                let res = tokio::time::timeout(
                    timeout_duration,
                    tokio::net::TcpStream::connect(&address),
                )
                .await;

                if let Ok(res) = res {
                    if res.is_ok() {
                        return true;
                    }
                };
                print_debug(format!(
                    "elapsed: {:?}, timeout {:?}",
                    now.elapsed(),
                    connection_check_timeout
                ));

                if now.elapsed() >= connection_check_timeout {
                    return false;
                };
                tokio::time::sleep(timeout_duration).await;
            }
            false
        }
        _ => {
            print_debug(format!(
                "the url {url} can not be parsed as a socket address"
            ));
            false
        }
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

/// Return the telemetry endpoint, defined by an environment variable
/// If the endpoint can be established with a secure channel to the project node, use that endpoint
/// otherwise use the HTTPS endpoint.
async fn opentelemetry_endpoint(
    cli_state: &CliState,
    ctx: &Context,
) -> Result<Option<TelemetryEndpoint>> {
    let route_and_identifier = match (
        telemetry_export_node_route()?,
        telemetry_export_node_identifier()?,
    ) {
        (Some(route), Some(identifier)) => Some((route, identifier)),
        _ => {
            if let Ok(project) = cli_state.projects().get_default_project().await {
                if project.is_ready() {
                    let via_project = is_exporting_via_project_set()?;
                    let via_authority = is_exporting_via_authority_set()? && !via_project;

                    if via_project {
                        print_debug("The project node is used as a telemetry endpoint");
                        Some((
                            project.project_multiaddr()?.clone(),
                            project.project_identifier().ok_or(ApiError::message(
                                "The default project must have an identifier",
                            ))?,
                        ))
                    } else if via_authority {
                        print_debug("The authority node is used as a telemetry endpoint");
                        Some((
                            project.authority_multiaddr()?.clone(),
                            project.authority_identifier().ok_or(ApiError::message(
                                "The default project authority must have an identifier",
                            ))?,
                        ))
                    } else {
                        print_debug(
                            "The default project is ready but export via the project node or the authority node is disabled. Getting the default HTTPs endpoint",
                        );
                        None
                    }
                } else {
                    print_debug(
                        "The default project is not ready. Getting the default HTTPs endpoint",
                    );
                    None
                }
            } else {
                print_debug("There is no default project. Getting the default HTTPs endpoint");
                None
            }
        }
    };

    let endpoint = if let Some((route, identifier)) = route_and_identifier {
        let client = make_secure_client(cli_state, ctx, route, identifier).await?;
        TelemetryEndpoint::SecureChannelEndpoint(client, telemetry_export_node_forwarder_service()?)
    } else {
        get_https_endpoint()?
    };
    print_debug(format!("Exporting telemetry data to: {endpoint}"));
    Ok(Some(endpoint))
}

async fn make_secure_client(
    cli_state: &CliState,
    ctx: &Context,
    route: MultiAddr,
    identifier: Identifier,
) -> Result<SecureClient> {
    let project_route = TransportRouteResolver::default()
        .allow_tcp()
        .resolve(&route)?;
    let default_node = if let Ok(node) = cli_state.get_default_node().await {
        node
    } else {
        cli_state
            .create_node_with_optional_identity(DEFAULT_NODE_NAME, &None)
            .await?
    };
    let secure_channels = cli_state.secure_channels(&default_node.name()).await?;

    Ok(SecureClient::new(
        secure_channels,
        None,
        TcpTransport::create(ctx)?,
        project_route,
        Arc::new(TrustIdentifierPolicy::new(identifier)),
        &default_node.identifier(),
        get_default_timeout(),
        get_default_timeout(),
    ))
}

/// Return the default HTTPs endpoint
pub fn get_https_endpoint() -> Result<TelemetryEndpoint> {
    Ok(TelemetryEndpoint::HttpsEndpoint(get_https_endpoint_url()?))
}

/// Return the default HTTPs endpoint URL
pub fn get_https_endpoint_url() -> Result<Url> {
    Ok(get_env_with_default::<UrlVar>(
        OCKAM_OPENTELEMETRY_ENDPOINT,
        UrlVar::from_string(DEFAULT_OPENTELEMETRY_ENDPOINT)?,
    )?
    .url)
}

/// Return true if the current user is an internal user
fn is_ockam_developer() -> Result<bool> {
    Ok(get_env_with_default(OCKAM_DEVELOPER, false)?)
}

/// Return the export timeout for spans, defined by an environment variable
pub fn span_export_timeout() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_SPAN_EXPORT_TIMEOUT,
        DEFAULT_EXPORT_TIMEOUT,
    )?)
}

/// Return the endpoint connection timeout, for a background node, defined by an environment variable
fn opentelemetry_endpoint_background_connection_timeout() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_BACKGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT,
        DEFAULT_TELEMETRY_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT,
    )?)
}

/// Return the endpoint connection timeout, for a foreground command, defined by an environment variable
fn opentelemetry_endpoint_foreground_connection_timeout() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_FOREGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT,
        DEFAULT_TELEMETRY_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT,
    )?)
}

/// Return the delay between the export of 2 spans batches, for a foreground command, defined by an environment variable
fn foreground_span_export_scheduled_delay() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY,
        DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
    )?)
}

/// Return the delay between the export of 2 spans batches, for a background node, defined by an environment variable
fn background_span_export_scheduled_delay() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_BACKGROUND_SPAN_EXPORT_SCHEDULED_DELAY,
        DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY,
    )?)
}

/// Return the size of the queue used to batch spans, defined by an environment variable
fn span_export_queue_size() -> Result<u16> {
    Ok(get_env_with_default(
        OCKAM_SPAN_EXPORT_QUEUE_SIZE,
        DEFAULT_SPAN_EXPORT_QUEUE_SIZE,
    )?)
}

/// Return the size of the queue used to batch log records, defined by an environment variable
fn log_export_queue_size() -> Result<u16> {
    Ok(get_env_with_default(
        OCKAM_LOG_EXPORT_QUEUE_SIZE,
        DEFAULT_LOG_EXPORT_QUEUE_SIZE,
    )?)
}

/// Return the export timeout for log records, defined by an environment variable
pub fn log_export_timeout() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_LOG_EXPORT_TIMEOUT,
        DEFAULT_EXPORT_TIMEOUT,
    )?)
}

/// Return the delay between the export of 2 logs batches, for a foreground command, defined by an environment variable
pub fn foreground_log_export_scheduled_delay() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_FOREGROUND_LOG_EXPORT_SCHEDULED_DELAY,
        DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
    )?)
}

/// Return the delay between the export of 2 logs batches, for a background node, defined by an environment variable
pub fn background_log_export_scheduled_delay() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_BACKGROUND_LOG_EXPORT_SCHEDULED_DELAY,
        DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY,
    )?)
}

/// Return the maximum time for sending log record batches when using a foreground node
pub fn foreground_log_export_cutoff() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_FOREGROUND_LOG_EXPORT_CUTOFF,
        DEFAULT_FOREGROUND_LOG_EXPORT_CUTOFF,
    )?)
}

/// Return the maximum time for sending span batches when using a foreground node
pub fn foreground_span_export_portal_cutoff() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_FOREGROUND_SPAN_EXPORT_CUTOFF,
        DEFAULT_FOREGROUND_SPAN_EXPORT_CUTOFF,
    )?)
}

/// Return the maximum time for sending log record batches when using a background node
pub fn background_log_export_cutoff() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_BACKGROUND_LOG_EXPORT_CUTOFF,
        DEFAULT_BACKGROUND_LOG_EXPORT_CUTOFF,
    )?)
}

/// Return the maximum time for sending span batches when using a background node
pub fn background_span_export_portal_cutoff() -> Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_BACKGROUND_SPAN_EXPORT_CUTOFF,
        DEFAULT_BACKGROUND_SPAN_EXPORT_CUTOFF,
    )?)
}
