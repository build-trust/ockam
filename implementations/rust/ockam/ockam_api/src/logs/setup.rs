use crate::journeys::APP_NAME;
use crate::logs::{LoggingConfiguration, TracingConfiguration};
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use gethostname::gethostname;
use ockam_core::env::FromString;
use ockam_node::Executor;
use opentelemetry::logs::{LogResult, Severity};
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::{global, Context, Key, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::export::logs::{LogData, LogExporter};
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::runtime::RuntimeChannel;
use opentelemetry_sdk::testing::trace::NoopSpanExporter;
use opentelemetry_sdk::trace::BatchConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{self as sdk};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use std::fmt::Debug;
use std::io::stdout;
use tonic::async_trait;
use tonic::metadata::*;
pub use tracing::level_filters::LevelFilter;
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const OCKAM_TRACER_NAME: &str = "ockam";

pub struct LoggingTracing;

impl LoggingTracing {
    /// Setup logging and tracing
    /// The app name is used to set an attribute on all events specifying if the event
    /// has been created by the cli or by a local node
    ///
    /// The TracingGuard is used to flush all events when dropped
    pub fn setup(
        logging_configuration: LoggingConfiguration,
        tracing_configuration: TracingConfiguration,
        app_name: &str,
    ) -> TracingGuard {
        // if the tracing configuration is enabled we use the
        // exporters. Those exporters can be decorated if we want to
        // intercept and debug the data they send, e.g.:
        // let decorated = DecoratedSpanExporter { exporter: span_exporter }
        let result = if tracing_configuration.is_enabled() {
            // create an exporter for spans
            // sending them to an OpenTelemetry collector using gRPC
            let span_exporter = Executor::execute_future(async move {
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(get_tracing_endpoint())
                    .with_metadata(get_otlp_headers())
                    .build_span_exporter()
                    .expect("failed to create the span exporter")
            })
            .expect("can't create a span exporter");

            // create an exporter for log records
            // sending them to an OpenTelemetry collector using gRPC
            let log_exporter = Executor::execute_future(async move {
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(get_tracing_endpoint())
                    .with_metadata(get_otlp_headers())
                    .build_log_exporter()
                    .expect("failed to create the log exporter")
            })
            .expect("can't create a log exporter");

            Self::setup_with_exporters(
                span_exporter,
                log_exporter,
                Some(BatchConfig::default()),
                logging_configuration.clone(),
                app_name,
            )
        } else {
            Self::setup_with_exporters(
                NoopSpanExporter::default(),
                NoopLogExporter,
                Some(BatchConfig::default()),
                logging_configuration.clone(),
                app_name,
            )
        };
        info!("tracing initialized");
        debug!("{:#?} {:#?}", logging_configuration, tracing_configuration);
        result
    }

    /// Setup the tracing and logging with some specific exporters
    ///  - the BatchConfig is used to possible send spans in batches
    ///  - the LoggingConfiguration is used to configure the logging layer
    ///    and the log files in particular
    pub fn setup_with_exporters<
        T: SpanExporter + Send + 'static,
        L: LogExporter + Send + 'static,
    >(
        span_exporter: T,
        log_exporter: L,
        batch_config: Option<BatchConfig>,
        logging_configuration: LoggingConfiguration,
        app_name: &str,
    ) -> TracingGuard {
        // Configure the tracing layer
        let app = app_name.to_string();
        let (tracing_layer, tracer_provider) = {
            // The setup of the tracer requires an async context
            Executor::execute_future(async move {
                let trace_config = sdk::trace::Config::default().with_resource(make_resource(app));
                let (tracer, tracer_provider) = create_tracer(
                    span_exporter,
                    Some(trace_config),
                    sdk::runtime::Tokio,
                    batch_config,
                );
                (
                    tracing_opentelemetry::layer().with_tracer(tracer),
                    tracer_provider,
                )
            })
            .expect("Failed to build the tracing layer")
        };

        // Configure the logging layer
        let app = app_name.to_string();
        let (logging_layer, logger_provider) = {
            Executor::execute_future(async move {
                let config = sdk::logs::Config::default().with_resource(make_resource(app));
                let provider = LoggerProvider::builder()
                    .with_config(config)
                    .with_batch_exporter(log_exporter, opentelemetry_sdk::runtime::Tokio)
                    .build();
                let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(
                    &provider,
                );
                (layer, provider)
            })
            .expect("Failed to build the logging layer")
        };

        // This propagator is used to encode the trace context data to strings
        // See OpenTelemetryContext for more details
        global::set_text_map_propagator(TraceContextPropagator::default());

        let subscriber = tracing_subscriber::registry()
            .with(logging_configuration.env_filter())
            .with(tracing_error::ErrorLayer::default())
            .with(logging_layer)
            .with(tracing_layer);

        if logging_configuration.is_enabled() {
            let (appender, guard) = match logging_configuration.log_dir() {
                // If a node dir path is not provided, log to stdout.
                None => {
                    let (n, guard) = tracing_appender::non_blocking(stdout());
                    let appender = layer()
                        .with_ansi(logging_configuration.is_colored())
                        .with_writer(n);
                    (Box::new(appender), guard)
                }
                // If a log directory is provided, log to a rolling file appender.
                Some(log_dir) => {
                    let r = RollingFileAppender::builder()
                        .rotation(Rotation::DAILY)
                        .max_log_files(logging_configuration.max_files())
                        .filename_prefix("stdout")
                        .filename_suffix("log")
                        .build(log_dir)
                        .expect("Failed to create rolling file appender");
                    let (n, guard) = tracing_appender::non_blocking(r);
                    let appender = layer().with_ansi(false).with_writer(n);
                    (Box::new(appender), guard)
                }
            };
            let res = match logging_configuration.format() {
                LogFormat::Pretty => subscriber.with(appender.pretty()).try_init(),
                LogFormat::Json => subscriber.with(appender.json()).try_init(),
                LogFormat::Default => subscriber.with(appender).try_init(),
            };
            res.expect("Failed to initialize tracing subscriber");

            TracingGuard {
                _worker_guard: Some(guard),
                logger_provider,
                tracer_provider,
            }
        } else {
            TracingGuard {
                _worker_guard: None,
                logger_provider,
                tracer_provider,
            }
        }
    }
}

fn create_tracer<S: SpanExporter + 'static, R: RuntimeChannel>(
    exporter: S,
    trace_config: Option<sdk::trace::Config>,
    runtime: R,
    batch_config: Option<BatchConfig>,
) -> (sdk::trace::Tracer, opentelemetry_sdk::trace::TracerProvider) {
    let mut provider_builder = sdk::trace::TracerProvider::builder();
    match batch_config {
        Some(batch_config) => {
            let span_processor = sdk::trace::BatchSpanProcessor::builder(exporter, runtime)
                .with_batch_config(batch_config)
                .build();
            provider_builder = provider_builder.with_span_processor(span_processor);
        }
        None => provider_builder = provider_builder.with_simple_exporter(exporter),
    };
    if let Some(config) = trace_config {
        provider_builder = provider_builder.with_config(config);
    }
    let provider = provider_builder.build();
    let tracer = provider.versioned_tracer(
        "ockam",
        Some(env!("CARGO_PKG_VERSION")),
        Some(SCHEMA_URL),
        None,
    );
    let _ = global::set_tracer_provider(provider.clone());
    (tracer, provider)
}

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

/// TODO: make sure that this is parsed as a proper URL
fn get_tracing_endpoint() -> String {
    match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) => {
            info!("OTEL_EXPORTER_OTLP_ENDPOINT defined as {endpoint}");
            endpoint
        }
        Err(_) => {
            info!("using the default value for OTEL_EXPORTER_OTLP_ENDPOINT: http://localhost:4317");
            "http://localhost:4317".to_string()
        }
    }
}

fn get_otlp_headers() -> MetadataMap {
    match std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
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

#[derive(Debug)]
pub struct TracingGuard {
    _worker_guard: Option<WorkerGuard>,
    logger_provider: LoggerProvider,
    tracer_provider: opentelemetry_sdk::trace::TracerProvider,
}

impl TracingGuard {
    pub fn force_flush(&self) {
        self.logger_provider.force_flush();
        self.tracer_provider.force_flush();
    }

    pub fn shutdown(&self) {
        self.force_flush();
        global::shutdown_tracer_provider();
        global::shutdown_logger_provider();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Default,
    Pretty,
    Json,
}

impl FromString for LogFormat {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "pretty" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            _ => Ok(LogFormat::Default),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogFormat::Default => write!(f, "default"),
            LogFormat::Pretty => write!(f, "pretty"),
            LogFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug)]
struct DecoratedLogExporter<L: LogExporter> {
    exporter: L,
}

#[async_trait]
impl<L: LogExporter> LogExporter for DecoratedLogExporter<L> {
    async fn export(&mut self, batch: Vec<LogData>) -> LogResult<()> {
        self.exporter.export(batch).await
    }

    fn shutdown(&mut self) {
        debug!("shutting down the log exporter");
        self.exporter.shutdown()
    }

    fn event_enabled(&self, level: Severity, target: &str, name: &str) -> bool {
        self.exporter.event_enabled(level, target, name)
    }
}

#[derive(Debug, Default)]
struct NoopLogExporter;

#[async_trait]
impl LogExporter for NoopLogExporter {
    async fn export(&mut self, _batch: Vec<LogData>) -> LogResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) {}

    fn event_enabled(&self, _level: Severity, _target: &str, _name: &str) -> bool {
        false
    }
}

#[derive(Debug)]
struct DecoratedSpanExporter<S: SpanExporter> {
    exporter: S,
}

#[async_trait]
impl<S: SpanExporter> SpanExporter for DecoratedSpanExporter<S> {
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        // debug!("exporting {} spans", batch.len());
        self.exporter.export(batch)
    }

    fn shutdown(&mut self) {
        debug!("shutting down the span exporter");
        self.exporter.shutdown()
    }

    fn force_flush(&mut self) -> BoxFuture<'static, ExportResult> {
        debug!("flushing the span exporter");
        self.exporter.force_flush()
    }
}

pub struct CurrentSpan;

impl CurrentSpan {
    pub fn set_attribute(name: &Key, value: &str) {
        Context::current()
            .span()
            .set_attribute(KeyValue::new(name.clone(), value.to_string()));
    }

    pub fn set_attribute_time(name: &Key) {
        let current_utc: DateTime<Utc> = Utc::now();
        let formatted_time: String = current_utc.format("%Y-%m-%dT%H:%M:%S.%3f").to_string();
        CurrentSpan::set_attribute(name, &formatted_time)
    }
}
