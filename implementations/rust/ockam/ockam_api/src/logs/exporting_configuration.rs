use crate::config::UrlVar;
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use crate::logs::ExportingEnabled;
use ockam_core::env::{get_env_with_default, FromString};
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
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
        Ok(ExportingConfiguration {
            enabled: exporting_enabled(
                quiet,
                opentelemetry_endpoint_foreground_connection_timeout()?,
            )?,
            span_export_timeout: span_export_timeout()?,
            log_export_timeout: span_export_timeout()?,
            span_export_scheduled_delay: foreground_span_export_scheduled_delay()?,
            log_export_scheduled_delay: foreground_log_export_scheduled_delay()?,
            opentelemetry_endpoint: opentelemetry_endpoint()?,
            is_ockam_developer: is_ockam_developer()?,
        })
    }

    /// Create a tracing configuration for a background node
    pub fn background(quiet: bool) -> ockam_core::Result<ExportingConfiguration> {
        Ok(ExportingConfiguration {
            enabled: exporting_enabled(
                quiet,
                opentelemetry_endpoint_background_connection_timeout()?,
            )?,
            span_export_timeout: span_export_timeout()?,
            log_export_timeout: log_export_timeout()?,
            span_export_scheduled_delay: background_span_export_scheduled_delay()?,
            log_export_scheduled_delay: background_log_export_scheduled_delay()?,
            opentelemetry_endpoint: opentelemetry_endpoint()?,
            is_ockam_developer: is_ockam_developer()?,
        })
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

/// Return true if tracing is enabled, as decided by the OCKAM_TRACING environment variable.
pub fn is_exporting_set() -> ockam_core::Result<bool> {
    get_env_with_default(OCKAM_OPENTELEMETRY_EXPORT, true)
}

/// Return ExportingEnabled::On if:
///
/// - Exporting has not been deactivated by the user
/// - The opentelemetry endpoint is accessible
///
fn exporting_enabled(
    quiet: bool,
    connection_check_timeout: Duration,
) -> ockam_core::Result<ExportingEnabled> {
    if !is_exporting_set()? {
        Ok(ExportingEnabled::Off)
    } else {
        let endpoint = opentelemetry_endpoint()?;
        if is_endpoint_accessible(endpoint.clone(), connection_check_timeout) {
            Ok(ExportingEnabled::On)
        } else {
            if !quiet {
                eprintln!("Exporting OpenTelemetry events is disabled because the OpenTelemetry collector endpoint at {} cannot be reached after {}ms", endpoint, connection_check_timeout.as_millis());
                eprintln!("You can disable the export of OpenTelemetry events with: `export OCKAM_OPENTELEMETRY_EXPORT=false` to avoid this connection check.");
            }
            Ok(ExportingEnabled::Off)
        }
    }
}

/// Return true if the endpoint can be accessed with a TCP connection
fn is_endpoint_accessible(url: Url, connection_check_timeout: Duration) -> bool {
    match to_socket_addr(url) {
        Some(address) => {
            std::net::TcpStream::connect_timeout(&address, connection_check_timeout).is_ok()
        }
        _ => false,
    }
}

/// Return a SocketAddr corresponding to the Url
fn to_socket_addr(url: Url) -> Option<SocketAddr> {
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
fn opentelemetry_endpoint() -> ockam_core::Result<Url> {
    Ok(get_env_with_default(
        OCKAM_OPENTELEMETRY_ENDPOINT,
        UrlVar::new(ExportingConfiguration::default_opentelemetry_endpoint()?),
    )?
    .url)
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
