use gethostname::gethostname;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::export::logs::LogExporter;
use opentelemetry_sdk::export::trace::SpanExporter;
use opentelemetry_sdk::logs::{BatchLogProcessor, LoggerProvider};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchConfig, BatchConfigBuilder, BatchSpanProcessor};
use opentelemetry_sdk::{self as sdk};
use opentelemetry_sdk::{logs, Resource};
use opentelemetry_semantic_conventions::attribute;
use std::io::{empty, stdout};
use tonic::metadata::*;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_core::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::fmt::format::{DefaultFields, Format};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, registry};

use crate::cli_state::journeys::APP_NAME;
use ockam_node::Executor;

use crate::logs::tracing_guard::TracingGuard;
use crate::logs::{
    ExportingConfiguration, GlobalErrorHandler, LoggingConfiguration, OckamLogExporter,
};
use crate::logs::{LogFormat, OckamSpanExporter};

pub struct LoggingTracing;

impl LoggingTracing {
    /// Setup logging and tracing
    /// The app name is used to set an attribute on all events specifying if the event
    /// has been created by the cli or by a local node.
    ///
    /// The TracingGuard is used to flush all events when dropped.
    pub fn setup(
        logging_configuration: &LoggingConfiguration,
        exporting_configuration: &ExportingConfiguration,
        app_name: &str,
        node_name: Option<String>,
    ) -> TracingGuard {
        if exporting_configuration.is_enabled() && logging_configuration.is_enabled() {
            // set-up logging and tracing
            Self::setup_with_exporters(
                create_span_exporter(exporting_configuration),
                create_log_exporter(exporting_configuration),
                logging_configuration,
                exporting_configuration,
                app_name,
                node_name,
            )
        } else if exporting_configuration.is_enabled() {
            Self::setup_tracing_only(
                create_span_exporter(exporting_configuration),
                logging_configuration,
                exporting_configuration,
                app_name,
                node_name,
            )
        } else {
            Self::setup_local_logging_only(logging_configuration)
        }
    }

    /// Setup the tracing and logging with some specific exporters
    ///  - the LoggingConfiguration is used to configure the logging layer and the log files in particular
    ///  - the Exporting configuration is used to send spans and log records to an OpenTelemetry collector
    pub fn setup_with_exporters<
        T: SpanExporter + Send + 'static,
        L: LogExporter + Send + 'static,
    >(
        span_exporter: T,
        log_exporter: L,
        logging_configuration: &LoggingConfiguration,
        exporting_configuration: &ExportingConfiguration,
        app_name: &str,
        node_name: Option<String>,
    ) -> TracingGuard {
        // configure the logging layer exporting OpenTelemetry log records
        let (logging_layer, logger_provider) =
            create_opentelemetry_logging_layer(app_name, exporting_configuration, log_exporter);

        // configure the tracing layer exporting OpenTelemetry spans
        let (tracing_layer, tracer_provider) = create_opentelemetry_tracing_layer(
            app_name,
            node_name,
            exporting_configuration,
            span_exporter,
        );

        // configure the appending layer, which outputs logs either to the console or to a file
        let (appender, worker_guard) = create_opentelemetry_appender(logging_configuration);

        // initialize the tracing subscriber with all the layers
        let layers = registry()
            .with(logging_configuration.env_filter())
            .with(tracing_error::ErrorLayer::default())
            .with(tracing_layer)
            .with(logging_layer);

        let result = match logging_configuration.format() {
            LogFormat::Pretty => layers.with(appender.pretty()).try_init(),
            LogFormat::Json => layers.with(appender.json()).try_init(),
            LogFormat::Default => layers.with(appender).try_init(),
        };
        result.expect("Failed to initialize tracing subscriber");

        // set the global settings:
        //   - the propagator is used to encode the trace context data to strings (see OpenTelemetryContext for more details)
        //   - the global error handler prints errors when exporting spans or log records fails
        global::set_text_map_propagator(TraceContextPropagator::default());
        set_global_error_handler(logging_configuration);

        TracingGuard::new(worker_guard, logger_provider, tracer_provider)
    }

    /// Setup logging to the console or to a file
    pub fn setup_local_logging_only(logging_configuration: &LoggingConfiguration) -> TracingGuard {
        let (appender, worker_guard) = make_logging_appender(logging_configuration);
        if logging_configuration.is_enabled() {
            let layers = registry().with(logging_configuration.env_filter());
            let result = match logging_configuration.format() {
                LogFormat::Pretty => layers.with(appender.pretty()).try_init(),
                LogFormat::Json => layers.with(appender.json()).try_init(),
                LogFormat::Default => layers.with(appender).try_init(),
            };
            result.expect("Failed to initialize tracing subscriber");
        };

        // the global error handler prints errors when exporting spans or log records fails
        set_global_error_handler(logging_configuration);

        TracingGuard::guard_only(worker_guard)
    }

    /// Setup the tracing a specific span exporter
    ///  - the LoggingConfiguration is used to filter spans (via its EnvFilter) and configure the global error handler
    ///  - the Exporting configuration is used to send spans and log records to an OpenTelemetry collector
    pub fn setup_tracing_only<T: SpanExporter + Send + 'static>(
        span_exporter: T,
        logging_configuration: &LoggingConfiguration,
        exporting_configuration: &ExportingConfiguration,
        app_name: &str,
        node_name: Option<String>,
    ) -> TracingGuard {
        let (tracing_layer, tracer_provider) = create_opentelemetry_tracing_layer(
            app_name,
            node_name,
            exporting_configuration,
            span_exporter,
        );

        // initialize the tracing subscriber with all the layers
        let result = registry()
            .with(logging_configuration.env_filter())
            .with(tracing_error::ErrorLayer::default())
            .with(tracing_layer)
            .try_init();

        result.expect("Failed to initialize tracing subscriber");

        // set the global settings:
        //   - the propagator is used to encode the trace context data to strings (see OpenTelemetryContext for more details)
        //   - the global error handler prints errors when exporting spans or log records fails
        global::set_text_map_propagator(TraceContextPropagator::default());
        set_global_error_handler(logging_configuration);

        TracingGuard::tracing_only(tracer_provider)
    }
}

// Create an exporter for log records
// They are sent to an OpenTelemetry collector using gRPC
fn create_log_exporter(
    exporting_configuration: &ExportingConfiguration,
) -> opentelemetry_otlp::LogExporter {
    let log_export_timeout = exporting_configuration.log_export_timeout();
    let endpoint = exporting_configuration.opentelemetry_endpoint().to_string();

    Executor::execute_future(async move {
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint)
            .with_timeout(log_export_timeout)
            .with_metadata(get_otlp_headers())
            .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
            .build_log_exporter()
            .expect("failed to create the log exporter")
    })
    .expect("can't create a log exporter")
}

/// Create a span exporter
// They are sent to an OpenTelemetry collector using gRPC
fn create_span_exporter(
    exporting_configuration: &ExportingConfiguration,
) -> opentelemetry_otlp::SpanExporter {
    let trace_export_timeout = exporting_configuration.span_export_timeout();
    let endpoint = exporting_configuration.opentelemetry_endpoint().to_string();

    Executor::execute_future(async move {
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint.clone())
            .with_timeout(trace_export_timeout)
            .with_metadata(get_otlp_headers())
            .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
            .build_span_exporter()
            .expect("failed to create the span exporter")
    })
    .expect("can't create a span exporter")
}

/// Create the tracing layer for OpenTelemetry
/// Spans are exported in batches
fn create_opentelemetry_tracing_layer<
    R: Subscriber + Send + 'static + for<'a> LookupSpan<'a>,
    S: SpanExporter + Send + 'static,
>(
    app_name: &str,
    node_name: Option<String>,
    exporting_configuration: &ExportingConfiguration,
    span_exporter: S,
) -> (
    OpenTelemetryLayer<R, sdk::trace::Tracer>,
    opentelemetry_sdk::trace::TracerProvider,
) {
    let app = app_name.to_string();
    let batch_config = BatchConfigBuilder::default()
        .with_max_export_timeout(exporting_configuration.span_export_timeout())
        .with_scheduled_delay(exporting_configuration.span_export_scheduled_delay())
        .with_max_concurrent_exports(8)
        .with_max_queue_size(exporting_configuration.span_export_queue_size() as usize)
        .build();
    let is_ockam_developer = exporting_configuration.is_ockam_developer();
    let span_export_cutoff = exporting_configuration.span_export_cutoff();
    Executor::execute_future(async move {
        let trace_config = sdk::trace::Config::default().with_resource(make_resource(app));
        let (tracer, tracer_provider) = create_tracer(
            trace_config,
            batch_config,
            OckamSpanExporter::new(
                span_exporter,
                node_name,
                is_ockam_developer,
                span_export_cutoff,
            ),
        );
        (
            tracing_opentelemetry::layer().with_tracer(tracer),
            tracer_provider,
        )
    })
    .expect("Failed to build the tracing layer")
}

/// Create the logging layer for OpenTelemetry
/// Log records are exported in batches
fn create_opentelemetry_logging_layer<L: LogExporter + Send + 'static>(
    app_name: &str,
    exporting_configuration: &ExportingConfiguration,
    log_exporter: L,
) -> (
    OpenTelemetryTracingBridge<LoggerProvider, logs::Logger>,
    LoggerProvider,
) {
    let app = app_name.to_string();
    let log_export_timeout = exporting_configuration.log_export_timeout();
    let log_export_scheduled_delay = exporting_configuration.log_export_scheduled_delay();
    let log_export_queue_size = exporting_configuration.log_export_queue_size();
    let log_export_cutoff = exporting_configuration.log_export_cutoff();
    Executor::execute_future(async move {
        let resource = make_resource(app);
        let batch_config = logs::BatchConfigBuilder::default()
            .with_max_export_timeout(log_export_timeout)
            .with_scheduled_delay(log_export_scheduled_delay)
            .with_max_queue_size(log_export_queue_size as usize)
            .build();

        let log_exporter = OckamLogExporter::new(log_exporter, log_export_cutoff);

        let log_processor =
            BatchLogProcessor::builder(log_exporter, opentelemetry_sdk::runtime::Tokio)
                .with_batch_config(batch_config)
                .build();
        let provider = LoggerProvider::builder()
            .with_resource(resource)
            .with_log_processor(log_processor)
            .build();
        let layer = OpenTelemetryTracingBridge::new(&provider);
        (layer, provider)
    })
    .expect("Failed to build the logging layer")
}

/// Create the appending layer for OpenTelemetry
fn create_opentelemetry_appender<S>(
    logging_configuration: &LoggingConfiguration,
) -> (
    fmt::Layer<S, DefaultFields, Format, NonBlocking>,
    WorkerGuard,
)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if logging_configuration.is_enabled() {
        make_logging_appender(logging_configuration)
    } else {
        // even if logging is not enabled, an empty writer is
        // necessary to make sure that all spans are emitted
        let (appender, worker_guard) = tracing_appender::non_blocking(empty());
        let appender = layer().with_writer(appender);
        (appender, worker_guard)
    }
}

/// Return either a console or a file appender for log messages
fn make_logging_appender<S>(
    logging_configuration: &LoggingConfiguration,
) -> (
    fmt::Layer<S, DefaultFields, Format, NonBlocking>,
    WorkerGuard,
)
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let layer = layer().with_ansi(logging_configuration.is_colored());
    let (writer, guard) = match logging_configuration.log_dir() {
        // If a node dir path is not provided, log to stdout.
        None => tracing_appender::non_blocking(stdout()),
        // If a log directory is provided, log to a rolling file appender.
        Some(log_dir) => {
            let r = RollingFileAppender::builder()
                .rotation(Rotation::DAILY)
                .max_log_files(logging_configuration.max_files() as usize)
                .filename_prefix("stdout")
                .filename_suffix("log")
                .build(log_dir)
                .expect("Failed to create rolling file appender");
            tracing_appender::non_blocking(r)
        }
    };
    (layer.with_writer(writer), guard)
}

/// Set a global error handler to report logging/tracing errors.
/// They are either:
///
///  - printed on the console
///  - logged to a log file
///  - not printed at all
///
fn set_global_error_handler(logging_configuration: &LoggingConfiguration) {
    if let Err(e) = match logging_configuration.global_error_handler() {
        GlobalErrorHandler::Off => global::set_error_handler(|_| ()).map_err(|e| format!("{e:?}")),
        GlobalErrorHandler::Console => global::set_error_handler(|e| println!("{e}"))
            .map_err(|e| format!("logging error: {e:?}")),
        GlobalErrorHandler::LogFile => match logging_configuration.log_dir() {
            Some(log_dir) => {
                use flexi_logger::*;
                let file_spec = FileSpec::default()
                    .directory(log_dir)
                    .basename("logging_tracing_errors");
                match Logger::try_with_str("info") {
                    Ok(logger) => {
                        // make sure that the log file is rolled every 3 days to avoid
                        // accumulating error messages
                        match logger
                            .log_to_file(file_spec)
                            .append()
                            .rotate(
                                Criterion::Age(Age::Day),
                                Naming::Timestamps,
                                Cleanup::KeepLogFiles(3),
                            )
                            .build()
                        {
                            Ok((log, _logger_handle)) => global::set_error_handler(move |e| {
                                log.log(
                                    &Record::builder()
                                        .level(Level::Error)
                                        .module_path(Some("ockam_api::logs::setup"))
                                        .args(format_args!("{e:?}"))
                                        .build(),
                                )
                            })
                            .map_err(|e| format!("{e:?}")),
                            Err(e) => Err(format!("{e:?}")),
                        }
                    }
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            None => {
                global::set_error_handler(|e| println!("ERROR! {e}")).map_err(|e| format!("{e:?}"))
            }
        },
    } {
        println!("cannot set a global error handler for logging: {e}");
    };
}

/// Create a Tracer using the provided span exporter
fn create_tracer<S: SpanExporter + 'static>(
    trace_config: sdk::trace::Config,
    batch_config: BatchConfig,
    exporter: S,
) -> (sdk::trace::Tracer, opentelemetry_sdk::trace::TracerProvider) {
    let span_processor = BatchSpanProcessor::builder(exporter, sdk::runtime::Tokio)
        .with_batch_config(batch_config)
        .build();
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_span_processor(span_processor)
        .with_config(trace_config)
        .build();
    let tracer = provider.tracer_builder("ockam").build();
    let _ = global::set_tracer_provider(provider.clone());
    (tracer, provider)
}

/// Make a resource representing the current application being traced.
/// The service name is used as a "dataset" by Honeycomb
fn make_resource(app_name: String) -> Resource {
    let host_name = gethostname().to_string_lossy().to_string();
    Resource::new(vec![
        KeyValue::new(attribute::SERVICE_NAME, "ockam"),
        KeyValue::new(attribute::HOST_NAME, host_name),
        KeyValue::new(APP_NAME.clone(), app_name),
    ])
}

/// This function can be used to pass the Honeycomb API key
/// from an environment variable if the OpenTelemetry collector endpoint is directly set to the Honeycomb API endpoint:
/// https://api.honeycomb.io:443/v1/traces
///
/// Then the OCKAM_OPENTELEMETRY_HEADERS variable can be defined as:
/// export OCKAM_OPENTELEMETRY_HEADERS="x-honeycomb-team=YOUR_API_KEY,x-honeycomb-dataset=YOUR_DATASET"
///
fn get_otlp_headers() -> MetadataMap {
    match std::env::var("OCKAM_OPENTELEMETRY_HEADERS") {
        Ok(headers) => {
            match headers.split_once('=') {
                // TODO: find a way to use a String as a key instead of a &'static str
                Some((key, value)) => {
                    match (
                        MetadataKey::from_bytes(key.as_bytes()),
                        MetadataValue::try_from(value.to_string()),
                    ) {
                        (Ok(key), Ok(value)) => {
                            let mut map = MetadataMap::with_capacity(1);
                            map.insert(key, value);
                            map
                        }
                        _ => MetadataMap::default(),
                    }
                }
                _ => MetadataMap::default(),
            }
        }
        _ => MetadataMap::default(),
    }
}
