use opentelemetry::global;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::trace::TracerProvider;
use tracing_appender::non_blocking::WorkerGuard;

/// The Tracing guard contains a guard closing the logging appender
/// and optionally the logger/tracer providers which can be used to force the flushing
/// of spans and log records
#[derive(Debug)]
pub struct TracingGuard {
    _worker_guard: Option<WorkerGuard>,
    logger_provider: Option<LoggerProvider>,
    tracer_provider: Option<TracerProvider>,
}

impl TracingGuard {
    /// Create a new tracing guard
    pub fn new(
        worker_guard: WorkerGuard,
        logger_provider: LoggerProvider,
        tracer_provider: TracerProvider,
    ) -> TracingGuard {
        TracingGuard {
            _worker_guard: Some(worker_guard),
            logger_provider: Some(logger_provider),
            tracer_provider: Some(tracer_provider),
        }
    }

    /// Create a Tracing guard when distributed tracing is deactivated
    pub fn guard_only(worker_guard: WorkerGuard) -> TracingGuard {
        TracingGuard {
            _worker_guard: Some(worker_guard),
            logger_provider: None,
            tracer_provider: None,
        }
    }

    /// Create a Tracing guard when only distributed tracing is activated
    pub fn tracing_only(tracer_provider: TracerProvider) -> TracingGuard {
        TracingGuard {
            _worker_guard: None,
            logger_provider: None,
            tracer_provider: Some(tracer_provider),
        }
    }

    pub async fn shutdown(self) {
        _ = tokio::task::spawn_blocking(|| {
            global::shutdown_tracer_provider();
        })
        .await
    }

    /// Export the current batches of spans and log records
    /// This is used right after a background node has started to get the first logs
    /// and in tests otherwise
    pub async fn force_flush(&self) {
        let logger_provider = self.logger_provider.clone();
        let tracer_provider = self.tracer_provider.clone();
        _ = tokio::task::spawn_blocking(|| {
            if let Some(logger_provider) = logger_provider {
                logger_provider.force_flush();
            }
            if let Some(tracer_provider) = tracer_provider {
                tracer_provider.force_flush();
            }
        })
        .await;
    }
}
