/// This module configures logging and tracing.
///
/// The logging/tracing code can be summarized as:
///
///  - `#[instrument]` creates spans.
//   - `info!`, `error!` etc... create log messages. ("log records" in the OpenTelemetry vocabulary)
//   - If -vvv is enabled or if OCKAM_LOGGING=true then the log messages end-up:
//      - In a log file for a background node.
//      - In the console for other commands.
//   - If OCKAM_TRACING=true then, _additionally_, the spans and logs messages are sent to an OpenTelemetry collector.
///
mod current_span;
mod default_values;
pub mod env_variables;
pub mod exporting_configuration;
mod log_exporters;
pub mod logging_configuration;
mod logging_options;
pub mod setup;
mod span_exporters;
mod tracing_guard;
mod tracing_options;

pub use current_span::*;
pub use exporting_configuration::*;
pub use log_exporters::*;
pub use logging_configuration::*;
pub use logging_options::*;
pub use setup::*;
pub use span_exporters::*;
pub use tracing_guard::*;
pub use tracing_options::*;
