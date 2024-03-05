use crate::logs::LogFormat;
use std::time::Duration;

///
/// LOGGING
///

/// Log format for files. See LogFormat for other values
pub(crate) const DEFAULT_LOG_FORMAT: LogFormat = LogFormat::Default;

/// Maximum size in Mb for a log file
pub(crate) const DEFAULT_LOG_MAX_SIZE_MB: u64 = 100;

/// Maximum number of files to create for a given node
pub(crate) const DEFAULT_LOG_MAX_FILES: u64 = 60;

/// Default endpoint for the OpenTelemetry collector
pub(crate) const DEFAULT_OPENTELEMETRY_ENDPOINT: &str =
    "http://k8s-opentele-otelcoll-aa527132c8-70cbeef1b85b559b.elb.us-west-1.amazonaws.com:4317/";

///
/// TRACING
///

/// Timeout for trying to access the OpenTelemetry collector endpoint when running a command
/// It is quite high but experimentation shows that sometimes there's quite some lag even if the endpoint is available
pub(crate) const DEFAULT_OPENTELEMETRY_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT: Duration =
    Duration::from_millis(500);

/// Timeout for trying to access the OpenTelemetry collector endpoint for a background
/// Since the node is going to run uninterrupted, we leave a longer amount of time than for a command
/// to try to reach the endpoint
pub(crate) const DEFAULT_OPENTELEMETRY_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT: Duration =
    Duration::from_secs(2);

/// Timeout for exporting spans or log records
pub(crate) const DEFAULT_EXPORT_TIMEOUT: Duration = Duration::from_secs(5);

// Maximum time between the export of batches. We set it high for a foreground task
// because we want the spans to be exported at the end of the execution
pub(crate) const DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY: Duration = Duration::from_secs(10);

// Maximum time between the export of batches
pub(crate) const DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY: Duration = Duration::from_secs(1);
