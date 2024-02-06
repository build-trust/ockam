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
pub(crate) const DEFAULT_OTEL_EXPORTER_OTLP_ENDPOINT: &str =
    "http://k8s-opentele-otelcoll-aa527132c8-70cbeef1b85b559b.elb.us-west-1.amazonaws.com:4317/";

///
/// TRACING
///

/// Timeout for trying to access the OpenTelemetry collector endpoint when running a command
/// It is quite high but experimentation shows that sometimes there's quite some lag even if the endpoint is available
pub(crate) const DEFAULT_TRACING_ENDPOINT_FOREGROUND_CONNECTION_TIMEOUT: Duration =
    Duration::from_millis(500);

/// Timeout for trying to access the OpenTelemetry collector endpoint for a background
/// Since the node is going to run uninterrupted, we leave a longer amount of time than for a command
/// to try to reach the endpoint
pub(crate) const DEFAULT_TRACING_ENDPOINT_BACKGROUND_CONNECTION_TIMEOUT: Duration =
    Duration::from_secs(2);

/// Timeout for exporting spans or log records
pub(crate) const DEFAULT_EXPORT_TIMEOUT: Duration = Duration::from_secs(5);

// Maximum time between the export of batches
// Important! For any foreground command execution
// we need to set a large scheduled delay for the batch log processor.
// Otherwise there can be a race condition where the 'ticks' from the scheduled exporting
// always get emitted and processed before the shutdown event has an opportunity to be enqueued.
// The result is that, when the collector endpoint is not responsive, a command stays stuck
// in an infinite loop.
pub(crate) const DEFAULT_FOREGROUND_EXPORT_SCHEDULED_DELAY: Duration = Duration::from_secs(1000);

// Maximum time between the export of batches
pub(crate) const DEFAULT_BACKGROUND_EXPORT_SCHEDULED_DELAY: Duration = Duration::from_secs(5);
