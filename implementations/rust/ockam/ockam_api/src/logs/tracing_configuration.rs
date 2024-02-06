use crate::config::{BooleanVar, DurationVar, UrlVar};
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use crate::logs::TracingEnabled;
use ockam_core::env::{get_env_with_default, FromString};
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use url::Url;

/// The tracing configuration contains all the parameters needed to configure the OpenTelemetry tracing layer.
///
/// Note: since this is the configuration for OpenTelemetry, this struct addresses the configuration
/// of both spans _and log records_ sent to an OpenTelemetry collector.
///
/// The configuration for log messages printed in a file, or in the console, use the LoggingConfiguration.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracingConfiguration {
    /// If TracingEnabled::On then spans and log records are sent to an OpenTelemetry collector.
    /// Some parameters for exporting the
    enabled: TracingEnabled,
    /// Maximum time for exporting a batch of spans
    trace_export_timeout: Duration,
    /// Maximum time for exporting a batch of log records
    log_export_timeout: Duration,
    /// Maximum time to wait until sending the current batch of spans
    trace_export_scheduled_delay: Duration,
    /// Maximum time to wait until sending the current batch of logs
    log_export_scheduled_delay: Duration,
    /// Url of the OpenTelemetry collector
    tracing_endpoint: Url,
}

impl TracingConfiguration {
    /// Return true if distributed tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled == TracingEnabled::On
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
    pub fn trace_export_timeout(&self) -> Duration {
        self.trace_export_timeout
    }

    /// Return the maximum time to wait until sending the current batch of spans
    pub fn trace_export_scheduled_delay(&self) -> Duration {
        self.trace_export_scheduled_delay
    }

    /// Return the URL where to export spans and log records
    pub fn tracing_endpoint(&self) -> Url {
        self.tracing_endpoint.clone()
    }

    /// Create a tracing configuration for a user command running in the foreground.
    /// (meaning that the process will shut down once the command has been executed)
    pub fn foreground(quiet: bool) -> ockam_core::Result<TracingConfiguration> {
        Ok(TracingConfiguration {
            enabled: tracing_enabled(quiet, tracing_endpoint_foreground_connection_timeout()?)?,
            trace_export_timeout: trace_export_timeout()?,
            log_export_timeout: trace_export_timeout()?,
            trace_export_scheduled_delay: trace_foreground_export_scheduled_delay()?,
            log_export_scheduled_delay: log_foreground_export_scheduled_delay()?,
            tracing_endpoint: tracing_endpoint()?,
        })
    }

    /// Create a tracing configuration for a background node
    pub fn background(quiet: bool) -> ockam_core::Result<TracingConfiguration> {
        Ok(TracingConfiguration {
            enabled: tracing_enabled(quiet, tracing_endpoint_background_connection_timeout()?)?,
            trace_export_timeout: trace_export_timeout()?,
            log_export_timeout: log_export_timeout()?,
            trace_export_scheduled_delay: trace_background_export_scheduled_delay()?,
            log_export_scheduled_delay: log_background_export_scheduled_delay()?,
            tracing_endpoint: tracing_endpoint()?,
        })
    }

    /// Create a a tracing configuration which is disabled
    pub fn off() -> ockam_core::Result<TracingConfiguration> {
        Ok(TracingConfiguration {
            enabled: TracingEnabled::Off,
            trace_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            log_export_timeout: DEFAULT_EXPORT_TIMEOUT,
            trace_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            log_export_scheduled_delay: DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY,
            tracing_endpoint: Self::default_otel_exporter_otlp_endpoint()?,
        })
    }

    /// Return the default endpoint for exporting traces
    fn default_otel_exporter_otlp_endpoint() -> ockam_core::Result<Url> {
        Ok(UrlVar::from_string(DEFAULT_OTEL_EXPORTER_OTLP_ENDPOINT)?.url)
    }
}

impl Display for TracingConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("tracing")
            .field("enabled", &self.enabled.to_string())
            .finish()
    }
}

/// Return true if tracing is enabled, as decided by the OCKAM_TRACING environment variable.
///
/// FOR NOW THE DEFAULT IS FALSE.
/// TODO: set it to true after enough testing!
///
pub fn is_tracing_set() -> ockam_core::Result<bool> {
    Ok(get_env_with_default(OCKAM_TRACING, BooleanVar("false".to_string()))?.is_true())
}

/// Return TracingEnabled::On if:
///
/// - Tracing has not been deactivated by the user
/// - The tracing endpoint is accessible
///
fn tracing_enabled(
    quiet: bool,
    connection_check_timeout: Duration,
) -> ockam_core::Result<TracingEnabled> {
    if !is_tracing_set()? {
        Ok(TracingEnabled::Off)
    } else {
        let endpoint = tracing_endpoint()?;
        if is_endpoint_accessible(endpoint.clone(), connection_check_timeout) {
            Ok(TracingEnabled::On)
        } else {
            if !quiet {
                println!("Tracing is disabled because the OpenTelemetry collector endpoint at {} cannot be reached after {}ms", endpoint, connection_check_timeout.as_millis());
                println!("You can disable tracing with: `export OCKAM_TRACING=false` to avoid this connection check.");
            }
            Ok(TracingEnabled::Off)
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
        _ => None,
    }
}

/// Return the tracing endpoint, defined by an environment variable
fn tracing_endpoint() -> ockam_core::Result<Url> {
    Ok(get_env_with_default(
        OCKAM_OTEL_EXPORTER_OTLP_ENDPOINT,
        UrlVar::new(TracingConfiguration::default_otel_exporter_otlp_endpoint()?),
    )?
    .url)
}

/// Return the export timeout for spans, defined by an environment variable
pub fn trace_export_timeout() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_TRACE_EXPORT_TIMEOUT,
        DurationVar::new(DEFAULT_EXPORT_TIMEOUT),
    )?
    .duration)
}

/// Return the endpoint connection timeout, for a background node, defined by an environment variable
fn tracing_endpoint_background_connection_timeout() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_TRACING_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT,
        DurationVar::new(DEFAULT_TRACING_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT),
    )?
    .duration)
}

/// Return the endpoint connection timeout, for a foreground command, defined by an environment variable
fn tracing_endpoint_foreground_connection_timeout() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_TRACING_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT,
        DurationVar::new(DEFAULT_TRACING_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT),
    )?
    .duration)
}

/// Return the delay between the export of 2 spans batches, for a foreground command, defined by an environment variable
fn trace_foreground_export_scheduled_delay() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_TRACE_FOREGROUND_EXPORT_SCHEDULED_DELAY,
        DurationVar::new(DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY),
    )?
    .duration)
}

/// Return the delay between the export of 2 spans batches, for a background node, defined by an environment variable
fn trace_background_export_scheduled_delay() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_TRACE_BACKGROUND_EXPORT_SCHEDULED_DELAY,
        DurationVar::new(DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY),
    )?
    .duration)
}

/// Return the export timeout for log records, defined by an environment variable
pub fn log_export_timeout() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_LOG_EXPORT_TIMEOUT,
        DurationVar::new(DEFAULT_EXPORT_TIMEOUT),
    )?
    .duration)
}

/// Return the delay between the export of 2 logs batches, for a foreground command, defined by an environment variable
pub fn log_foreground_export_scheduled_delay() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_LOG_FOREGROUND_EXPORT_SCHEDULED_DELAY,
        DurationVar::new(DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY),
    )?
    .duration)
}

/// Return the delay between the export of 2 logs batches, for a background node, defined by an environment variable
pub fn log_background_export_scheduled_delay() -> ockam_core::Result<Duration> {
    Ok(get_env_with_default(
        OCKAM_LOG_BACKGROUND_EXPORT_SCHEDULED_DELAY,
        DurationVar::new(DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY),
    )?
    .duration)
}
