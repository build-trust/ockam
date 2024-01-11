use crate::logs::env::{log_format, log_max_files};
use ockam_core::async_trait;
use ockam_core::env::FromString;
use ockam_node::Executor;
use opentelemetry::logs::{LogResult, Severity};
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::export::logs::{LogData, LogExporter};
use opentelemetry_sdk::export::trace::SpanExporter;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::runtime::RuntimeChannel;
use opentelemetry_sdk::trace::BatchConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{self as sdk};
use opentelemetry_semantic_conventions::SCHEMA_URL;
use std::io::stdout;
use std::path::PathBuf;
pub use tracing::level_filters::LevelFilter;
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod env;

pub struct Logging;

impl Logging {
    pub fn setup(
        level: LevelFilter,
        color: bool,
        node_dir: Option<PathBuf>,
        crates: &[&str],
    ) -> Option<LoggingGuard> {
        #[cfg(feature = "opentelemetry")]
        let span_exporter = Executor::execute_future(async move {
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(get_tracing_endpoint())
                .build_span_exporter()
                .expect("failed to create the span exporter")
        })
        .expect("can't create a span exporter");

        #[cfg(feature = "opentelemetry")]
        let log_exporter = Executor::execute_future(async move {
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(get_tracing_endpoint())
                .build_log_exporter()
                .expect("failed to create the log exporter")
        })
        .expect("can't create a log exporter");

        #[cfg(feature = "opentelemetry")]
        let batch_config = sdk::trace::BatchConfig::default();

        Self::setup_with_exporters(
            #[cfg(feature = "opentelemetry")]
            span_exporter,
            #[cfg(feature = "opentelemetry")]
            log_exporter,
            #[cfg(feature = "opentelemetry")]
            Some(batch_config),
            level,
            color,
            node_dir,
            crates,
        )
    }

    pub fn setup_with_exporters<
        T: SpanExporter + Send + 'static,
        L: LogExporter + Send + 'static,
    >(
        #[cfg(feature = "opentelemetry")] span_exporter: T,
        #[cfg(feature = "opentelemetry")] log_exporter: L,
        #[cfg(feature = "opentelemetry")] batch_config: Option<BatchConfig>,
        level: LevelFilter,
        color: bool,
        node_dir: Option<PathBuf>,
        crates: &[&str],
    ) -> Option<LoggingGuard> {
        #[cfg(feature = "opentelemetry")]
        let tracing_layer = {
            // we don't really need to execute a future here but this
            // call will initialize the tokio runtime which is necessary for setting-up the exporter
            Executor::execute_future(async move {
                let trace_config =
                    sdk::trace::Config::default().with_resource(Resource::new(vec![
                        KeyValue::new(
                            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                            "ockam",
                        ),
                    ]));

                let tracer = create_tracer(
                    span_exporter,
                    Some(trace_config),
                    sdk::runtime::Tokio,
                    batch_config,
                );
                tracing_opentelemetry::layer().with_tracer(tracer)
            })
            .expect("Failed to build the tracing layer")
        };

        #[cfg(not(feature = "opentelemetry"))]
        let tracing_layer = NopLayer;

        #[cfg(feature = "opentelemetry")]
        let (logging_layer, provider) = {
            Executor::execute_future(async move {
                let config =
                    sdk::logs::Config::default().with_resource(Resource::new(vec![KeyValue::new(
                        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                        "ockam",
                    )]));

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

        #[cfg(not(feature = "opentelemetry"))]
        let logging_layer = NopLayer;

        let filter = {
            let builder = EnvFilter::builder();
            builder.with_default_directive(level.into()).parse_lossy(
                crates
                    .iter()
                    .map(|c| format!("{c}={level}"))
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };

        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(tracing_error::ErrorLayer::default())
            .with(logging_layer)
            .with(tracing_layer);

        let (appender, guard) = match node_dir {
            // If a node dir path is not provided, log to stdout.
            None => {
                let (n, guard) = tracing_appender::non_blocking(stdout());
                let appender = layer().with_ansi(color).with_writer(n);
                (Box::new(appender), guard)
            }
            // If a log path is provided, log to a rolling file appender.
            Some(node_dir) => {
                let r = RollingFileAppender::builder()
                    .rotation(Rotation::DAILY)
                    .max_log_files(log_max_files())
                    .filename_prefix("stdout")
                    .filename_suffix("log")
                    .build(node_dir)
                    .expect("Failed to create rolling file appender");
                let (n, guard) = tracing_appender::non_blocking(r);
                let appender = layer().with_ansi(false).with_writer(n);
                (Box::new(appender), guard)
            }
        };
        let res = match log_format() {
            LogFormat::Pretty => subscriber.with(appender.pretty()).try_init(),
            LogFormat::Json => subscriber.with(appender.json()).try_init(),
            LogFormat::Default => subscriber.with(appender).try_init(),
        };
        res.expect("Failed to initialize tracing subscriber");

        info!("tracing initialized");
        Some(LoggingGuard {
            _worker_guard: guard,
            logger_provider: provider,
        })
    }
}

fn create_tracer<S: SpanExporter + 'static, R: RuntimeChannel>(
    exporter: S,
    trace_config: Option<sdk::trace::Config>,
    runtime: R,
    batch_config: Option<BatchConfig>,
) -> sdk::trace::Tracer {
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
    let _ = global::set_tracer_provider(provider);
    tracer
}

/// TODO: make sure that this is parsed as a proper URL
fn get_tracing_endpoint() -> String {
    std::env::var("OCKAM_TRACING_GRPC_ENDPOINT").unwrap_or("http://localhost:4317".to_string())
}

#[derive(Debug)]
pub struct LoggingGuard {
    _worker_guard: WorkerGuard,
    logger_provider: LoggerProvider,
}

impl LoggingGuard {
    pub fn force_flush(&self) {
        self.logger_provider.force_flush();
    }

    pub fn shutdown(&self) {
        self.force_flush();
        global::shutdown_tracer_provider();
        global::shutdown_logger_provider();
    }
}

#[derive(Clone)]
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

#[cfg(test)]
mod tests {
    use crate::logs::{LevelFilter, Logging};
    use crate::random_name;
    use opentelemetry::global;
    use opentelemetry::trace::Tracer;
    use opentelemetry_sdk::{self as sdk};
    use sdk::testing::logs::*;
    use sdk::testing::trace::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_log_and_traces() {
        let temp_file = NamedTempFile::new().unwrap();
        let log_directory = &temp_file.path().parent().unwrap().join(random_name());

        let spans_exporter = InMemorySpanExporter::default();
        let logs_exporter = InMemoryLogsExporter::default();
        let ockam_crates = &["ockam_api"];
        let guard = Logging::setup_with_exporters(
            spans_exporter.clone(),
            logs_exporter.clone(),
            None,
            LevelFilter::TRACE,
            false,
            Some(log_directory.into()),
            ockam_crates,
        )
        .unwrap();

        let tracer = global::tracer("ockam");
        tracer.in_span("Logging::test", |_| {
            info!("inside span");
            error!("something went wrong!");
        });

        // check that the spans are exported
        guard.force_flush();
        let spans = spans_exporter.get_finished_spans().unwrap();
        assert_eq!(spans.len(), 1);
        let parent_span = spans.first().unwrap();

        // check that log records are exported
        let logs = logs_exporter.get_emitted_logs().unwrap();
        assert_eq!(logs.len(), 3);
        for log in logs {
            assert_eq!(
                log.clone().record.trace_context.map(|tc| tc.trace_id),
                Some(parent_span.span_context.trace_id()),
                "{log:?}\n{parent_span:?}"
            )
        }

        // read the content of the log file to make sure that log messages are there
        let mut stdout_file_checked = false;
        for file in fs::read_dir(log_directory).unwrap() {
            let file_path = file.unwrap().path();
            if file_path.to_string_lossy().contains("stdout") {
                let contents = fs::read_to_string(file_path).unwrap();
                assert!(
                    contents.contains("INFO ockam_api::logs::tests: inside span"),
                    "{:?}",
                    contents
                );
                assert!(
                    contents.contains("ERROR ockam_api::logs::tests: something went wrong!"),
                    "{:?}",
                    contents
                );
                stdout_file_checked = true
            }
        }

        assert!(
            stdout_file_checked,
            "the stdout log file must have been found and checked"
        )
    }
}

#[derive(Debug)]
struct LogsCollector {
    batch: Vec<LogData>,
}

#[async_trait]
impl LogExporter for LogsCollector {
    async fn export(&mut self, batch: Vec<LogData>) -> LogResult<()> {
        self.batch.extend(batch);
        Ok(())
    }

    fn shutdown(&mut self) {}

    fn event_enabled(&self, _level: Severity, _target: &str, _name: &str) -> bool {
        true
    }
}
