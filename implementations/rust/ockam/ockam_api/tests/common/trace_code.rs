use crate::common::test_spans::Trace;
use ockam_api::logs::{ExportingConfiguration, LoggingConfiguration, LoggingTracing};
use ockam_api::CliState;
use ockam_core::{AsyncTryClone, OpenTelemetryContext};
use ockam_node::{Context, NodeBuilder};
use opentelemetry::global;
use opentelemetry::trace::{FutureExt, Tracer};
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::testing::logs::InMemoryLogsExporter;
use opentelemetry_sdk::testing::trace::InMemorySpanExporter;
use std::future::Future;

/// Run an async function using a tracer and return:
///
///  - the return value of the function
///  - all the exported spans
pub fn trace_code<F>(
    state: &CliState,
    f: impl Fn(Context) -> F + Send + Sync + 'static,
) -> (F::Output, Vec<SpanData>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let spans_exporter = InMemorySpanExporter::default();
    let guard = LoggingTracing::setup_with_exporters(
        spans_exporter.clone(),
        InMemoryLogsExporter::default(),
        &LoggingConfiguration::off()
            .unwrap()
            .set_all_crates()
            .set_log_level(tracing_core::metadata::Level::TRACE),
        &ExportingConfiguration::foreground(state).unwrap(),
        "test",
        None,
    );

    let (mut ctx, mut executor) = NodeBuilder::new().build();

    let tracer = global::tracer("ockam-test");
    let result = executor
        .execute_no_abort(async move {
            let result = tracer
                .in_span("root", |_| {
                    ctx.set_tracing_context(OpenTelemetryContext::current());
                    async { f(ctx.async_try_clone().await.unwrap()).await }.with_current_context()
                })
                .await;
            let _ = ctx.stop().await;
            result
        })
        .unwrap();

    // get the exported spans
    guard.force_flush();
    let spans = spans_exporter.get_finished_spans().unwrap();
    (result, spans)
}

/// Return a string displaying the traces for all the given spans
pub fn display_traces(spans: Vec<SpanData>) -> String {
    let mut traces = Trace::from_span_data(spans);
    traces.sort_by_key(|t| t.to_string());

    // remove some uninteresting traces based on their root name
    traces.retain(|t| {
        !["start", "shutdown", "initialize", "process"]
            .iter()
            .any(|v| t.0.to_string().split('\n').collect::<Vec<_>>()[0].ends_with(v))
    });

    traces
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
