use gethostname::gethostname;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::export::logs::LogExporter;
use opentelemetry_sdk::export::trace::SpanExporter;
use opentelemetry_sdk::logs::{BatchLogProcessor, LoggerProvider};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchConfig, BatchSpanProcessor};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{self as sdk};
use opentelemetry_semantic_conventions::SCHEMA_URL;
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

use ockam_node::Executor;

use crate::journeys::APP_NAME;
use crate::logs::log_processors::NonBlockingLogProcessor;
use crate::logs::span_processors::NonBlockingSpanProcessor;
use crate::logs::tracing_guard::TracingGuard;
use crate::logs::{DecoratedSpanExporter, LogFormat};
use crate::logs::{GlobalErrorHandler, LoggingConfiguration, TracingConfiguration};

pub struct LoggingTracing;

impl LoggingTracing {
    /// Setup logging and tracing
    /// The app name is used to set an attribute on all events specifying if the event
    /// has been created by the cli or by a local node.
    ///
    /// The TracingGuard is used to flush all events when dropped.
    pub fn setup(
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &TracingConfiguration,
        app_name: &str,
    ) -> TracingGuard {
        // if the tracing configuration is enabled we use the
        // exporters.
        //
        // For debugging those exporters can be decorated if we want to
        // intercept and print some of the data they send, e.g.:
        // let decorated = DecoratedSpanExporter { exporter: span_exporter }
        if tracing_configuration.is_enabled() && logging_configuration.is_enabled() {
            // set-up logging and tracing
            Self::setup_with_exporters(
                DecoratedSpanExporter {
                    exporter: create_span_exporter(tracing_configuration),
                },
                create_log_exporter(tracing_configuration),
                logging_configuration,
                tracing_configuration,
                app_name,
            )
        } else if tracing_configuration.is_enabled() {
            Self::setup_tracing_only(
                create_span_exporter(tracing_configuration),
                logging_configuration,
                tracing_configuration,
                app_name,
            )
        } else {
            Self::setup_local_logging_only(logging_configuration)
        }
    }

    /// Setup the tracing and logging with some specific exporters
    ///  - the BatchConfig is used to send spans in batches
    ///  - the LoggingConfiguration is used to configure the logging layer
    ///    and the log files in particular
    pub fn setup_with_exporters<
        T: SpanExporter + Send + 'static,
        L: LogExporter + Send + 'static,
    >(
        span_exporter: T,
        log_exporter: L,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &TracingConfiguration,
        app_name: &str,
    ) -> TracingGuard {
        // configure the logging layer exporting OpenTelemetry log records
        let (logging_layer, logger_provider) =
            create_opentelemetry_logging_layer(app_name, tracing_configuration, log_exporter);

        // configure the tracing layer exporting OpenTelemetry spans
        let (tracing_layer, tracer_provider) =
            create_opentelemetry_tracing_layer(app_name, tracing_configuration, span_exporter);

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
    ///  - the BatchConfig is used to send spans in batches
    ///  - the LoggingConfiguration contains a filter that is common to spans and logs
    pub fn setup_tracing_only<T: SpanExporter + Send + 'static>(
        span_exporter: T,
        logging_configuration: &LoggingConfiguration,
        tracing_configuration: &TracingConfiguration,
        app_name: &str,
    ) -> TracingGuard {
        // configure the tracing layer exporting OpenTelemetry spans
        let (tracing_layer, tracer_provider) =
            create_opentelemetry_tracing_layer(app_name, tracing_configuration, span_exporter);

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

fn create_log_exporter(
    tracing_configuration: &TracingConfiguration,
) -> opentelemetry_otlp::LogExporter {
    // create an exporter for log records
    // sending them to an OpenTelemetry collector using gRPC
    let log_export_timeout = tracing_configuration.log_export_timeout();
    let tracing_endpoint = tracing_configuration.tracing_endpoint().to_string();

    Executor::execute_future(async move {
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(tracing_endpoint)
            .with_metadata(get_otlp_headers())
            .with_timeout(log_export_timeout)
            .build_log_exporter()
            .expect("failed to create the log exporter")
    })
    .expect("can't create a log exporter")
}

/// Create a span exporter
fn create_span_exporter(
    tracing_configuration: &TracingConfiguration,
) -> opentelemetry_otlp::SpanExporter {
    // create an exporter for spans
    // sending them to an OpenTelemetry collector using gRPC
    let trace_export_timeout = tracing_configuration.trace_export_timeout();
    let tracing_endpoint = tracing_configuration.tracing_endpoint().to_string();

    Executor::execute_future(async move {
        opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(tracing_endpoint.clone())
            .with_metadata(get_otlp_headers())
            .with_timeout(trace_export_timeout)
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
    tracing_configuration: &TracingConfiguration,
    span_exporter: S,
) -> (
    OpenTelemetryLayer<R, sdk::trace::Tracer>,
    opentelemetry_sdk::trace::TracerProvider,
) {
    let app = app_name.to_string();
    let batch_config = BatchConfig::default()
        .with_max_export_timeout(tracing_configuration.trace_export_timeout())
        .with_scheduled_delay(tracing_configuration.trace_export_scheduled_delay())
        .with_max_concurrent_exports(4);
    Executor::execute_future(async move {
        let trace_config = sdk::trace::Config::default().with_resource(make_resource(app));
        let (tracer, tracer_provider) = create_tracer(trace_config, batch_config, span_exporter);
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
    tracing_configuration: &TracingConfiguration,
    log_exporter: L,
) -> (
    OpenTelemetryTracingBridge<LoggerProvider, opentelemetry_sdk::logs::Logger>,
    LoggerProvider,
) {
    let app = app_name.to_string();
    let log_export_timeout = tracing_configuration.log_export_timeout();
    let log_export_scheduled_delay = tracing_configuration.log_export_scheduled_delay();
    Executor::execute_future(async move {
        let config = sdk::logs::Config::default().with_resource(make_resource(app));
        let log_processor =
            BatchLogProcessor::builder(log_exporter, opentelemetry_sdk::runtime::Tokio)
                .with_max_timeout(log_export_timeout)
                .with_scheduled_delay(log_export_scheduled_delay)
                .build();
        let provider = LoggerProvider::builder()
            .with_config(config)
            .with_log_processor(NonBlockingLogProcessor::new(log_processor))
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
///  - logged to the current log file
///  - not printed at all
///
fn set_global_error_handler(logging_configuration: &LoggingConfiguration) {
    if let Err(e) = match logging_configuration.global_error_handler() {
        GlobalErrorHandler::Off => global::set_error_handler(|_| ()),
        GlobalErrorHandler::Console => global::set_error_handler(|e| println!("{e}")),
        GlobalErrorHandler::LogFile => global::set_error_handler(move |e| error!("{e}")),
    } {
        println!("cannot set a error handler for logging: {e}");
    }
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
        .with_span_processor(NonBlockingSpanProcessor::new(span_processor))
        .with_config(trace_config)
        .build();
    let tracer = provider.versioned_tracer(
        "ockam",
        Some(env!("CARGO_PKG_VERSION")),
        Some(SCHEMA_URL),
        None,
    );
    let _ = global::set_tracer_provider(provider.clone());
    (tracer, provider)
}

/// Make a resource representing the current application being traced.
/// The service name is used as a "dataset" by Honeycomb
fn make_resource(app_name: String) -> Resource {
    let host_name = gethostname().to_string_lossy().to_string();
    Resource::new(vec![
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            "ockam",
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::HOST_NAME,
            host_name,
        ),
        KeyValue::new(APP_NAME.clone(), app_name),
    ])
}

/// This function can be used to pass the Honeycomb API key
/// from an environment variable if the OpenTelemetry collector endpoint is directly set to the Honeycomb API endpoint:
/// https://api.honeycomb.io:443/v1/traces
///
/// Then the OCKAM_OTEL_EXPORTER_OTLP_HEADERS variable can be defined as:
/// export OCKAM_OTEL_EXPORTER_OTLP_HEADERS="x-honeycomb-team=YOUR_API_KEY,x-honeycomb-dataset=YOUR_DATASET"
///
fn get_otlp_headers() -> MetadataMap {
    match std::env::var("OCKAM_OTEL_EXPORTER_OTLP_HEADERS") {
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
