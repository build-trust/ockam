///
/// LOGGING CONFIGURATION
///

/// Decides if logs should be created. Accepted values, see FromString<bool>. For example; true, false, 1, 0
pub(crate) const OCKAM_LOGGING: &str = "OCKAM_LOGGING";

/// Required log level. Accepted values, see LevelVar. For example: trace, debug, info, warn, error
pub(crate) const OCKAM_LOG_LEVEL: &str = "OCKAM_LOG_LEVEL";

///
/// LOG FILE CONFIGURATION
///

/// Max size of a log file before it is rotated, in Mb
pub(crate) const OCKAM_LOG_MAX_SIZE_MB: &str = "OCKAM_LOG_MAX_SIZE_MB";

/// Maximum number of log files created per node
pub(crate) const OCKAM_LOG_MAX_FILES: &str = "OCKAM_LOG_MAX_FILES";

/// Log format. Accepted values, see LogFormat. For example: pretty, json, default
pub(crate) const OCKAM_LOG_FORMAT: &str = "OCKAM_LOG_FORMAT";

/// Filter for log messages based on crate names. Accepted values: 'all', 'default', 'comma-separated strings'. For example: ockam_core,ockam_api
pub(crate) const OCKAM_LOG_CRATES_FILTER: &str = "OCKAM_LOG_CRATES_FILTER";

///
/// TRACING CONFIGURATION
///

/// Decides if spans and log records should be created and exported. Accepted values, see BooleanVar. For example; true, false, 1, 0
pub(crate) const OCKAM_TELEMETRY_EXPORT: &str = "OCKAM_TELEMETRY_EXPORT";

/// Deprecated, use OCKAM_TELEMETRY_EXPORT instead
pub(crate) const OCKAM_OPENTELEMETRY_EXPORT: &str = "OCKAM_OPENTELEMETRY_EXPORT";

/// Decides if spans and log records should be exported via the project exporter portal. Accepted values, see BooleanVar. For example; true, false, 1, 0
pub(crate) const OCKAM_TELEMETRY_EXPORT_VIA_PORTAL: &str = "OCKAM_TELEMETRY_EXPORT_VIA_PORTAL";

/// Boolean set to true if the current user is an Ockam developer
pub(crate) const OCKAM_DEVELOPER: &str = "OCKAM_DEVELOPER";

/// If this variable is true, print statements will debug the setting of the OpenTelemetry export
pub(crate) const OCKAM_OPENTELEMETRY_EXPORT_DEBUG: &str = "OCKAM_OPENTELEMETRY_EXPORT_DEBUG";

///
/// TELEMETRY COLLECTOR ENDPOINT CONFIGURATION
///

/// URL for the OpenTelemetry collector. Accepted values, see UrlVar. For example: http://127.0.0.1:4317
pub(crate) const OCKAM_OPENTELEMETRY_ENDPOINT: &str = "OCKAM_OPENTELEMETRY_ENDPOINT";

/// Timeout for trying to connect to the endpoint before deciding that exporting traces
/// from a foreground command will not be possible. For example: 500ms
pub(crate) const OCKAM_FOREGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT: &str =
    "OCKAM_FOREGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT";

/// Timeout for trying to connect to the endpoint before deciding that exporting traces
/// from a background node will not be possible. Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_BACKGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT: &str =
    "OCKAM_BACKGROUND_TELEMETRY_ENDPOINT_CONNECTION_TIMEOUT";

///
/// TELEMETRY COLLECTOR EXPORT CONFIGURATION
///

/// Name of the background node used to export Telemetry traces
pub(crate) const OCKAM_TELEMETRY_NODE_NAME: &str = "ockam-telemetry-inlet";

/// Name of the inlet used to export Telemetry traces
pub(crate) const OCKAM_TELEMETRY_INLET_ALIAS: &str = "ockam-telemetry";

/// Name of the relay used to export Telemetry traces
pub(crate) const OCKAM_TELEMETRY_RELAY_NAME: &str = "ockam-telemetry";

/// Timeout for trying to export spans to the endpoint.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_SPAN_EXPORT_TIMEOUT: &str = "OCKAM_SPAN_EXPORT_TIMEOUT";

/// Timeout for trying to export log records to the endpoint.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_LOG_EXPORT_TIMEOUT: &str = "OCKAM_LOG_EXPORT_TIMEOUT";

/// Timeout for exporting the current batch of spans to the endpoint, when running a command.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY: &str =
    "OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY";

/// Timeout for exporting the current batch of spans to the endpoint, when running a background node.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_BACKGROUND_SPAN_EXPORT_SCHEDULED_DELAY: &str =
    "OCKAM_BACKGROUND_SPAN_EXPORT_SCHEDULED_DELAY";

/// Timeout for exporting the current batch of log records to the endpoint, when running a command.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_FOREGROUND_LOG_EXPORT_SCHEDULED_DELAY: &str =
    "OCKAM_FOREGROUND_LOG_EXPORT_SCHEDULED_DELAY";

/// Timeout for exporting the current batch of log records to the endpoint, when running a background node.
/// Accepted values, see DurationVar. For example: 500ms
pub(crate) const OCKAM_BACKGROUND_LOG_EXPORT_SCHEDULED_DELAY: &str =
    "OCKAM_BACKGROUND_LOG_EXPORT_SCHEDULED_DELAY";

/// Size of the queue used to batch spans.
/// Accepted values, u16. For example: 2048
pub(crate) const OCKAM_SPAN_EXPORT_QUEUE_SIZE: &str = "OCKAM_SPAN_EXPORT_QUEUE_SIZE";

/// Size of the queue used to batch log records.
/// Accepted values, u16. For example: 2048
pub(crate) const OCKAM_LOG_EXPORT_QUEUE_SIZE: &str = "OCKAM_LOG_EXPORT_QUEUE_SIZE";

/// Maximum time for sending a log batch and not waiting for a response when running
/// a foreground command export log records. For example: 200ms
pub(crate) const OCKAM_FOREGROUND_LOG_EXPORT_CUTOFF: &str = "OCKAM_FOREGROUND_LOG_EXPORT_CUTOFF";

/// Maximum time for sending a span batch and not waiting for a response when running
/// a foreground command to export span batches. For example: 200ms
pub(crate) const OCKAM_FOREGROUND_SPAN_EXPORT_CUTOFF: &str = "OCKAM_FOREGROUND_SPAN_EXPORT_CUTOFF";

/// Maximum time for sending a log batch and not waiting for a response when running
/// a background command to export log records. For example: 200ms
pub(crate) const OCKAM_BACKGROUND_LOG_EXPORT_CUTOFF: &str = "OCKAM_BACKGROUND_LOG_EXPORT_CUTOFF";

/// Maximum time for sending a span batch and not waiting for a response when running
/// a background command to export span batches. For example: 200ms
pub(crate) const OCKAM_BACKGROUND_SPAN_EXPORT_CUTOFF: &str = "OCKAM_BACKGROUND_SPAN_EXPORT_CUTOFF";

///
/// TELEMETRY COLLECTOR ERRORS CONFIGURATION
///

/// Global error handler for the tracing crate
/// Accepted values, see GlobalErrorHandler. For example: off, console, logfile
pub(crate) const OCKAM_TRACING_GLOBAL_ERROR_HANDLER: &str = "OCKAM_TRACING_GLOBAL_ERROR_HANDLER";
