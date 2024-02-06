use opentelemetry::trace::TraceResult;
use opentelemetry::Context;
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::trace::{Span, SpanProcessor};

/// This SpanProcessor does not perform any shutdown
/// because there is a deadlock condition in the opentelemetry library on shutdown
#[derive(Debug)]
pub struct NonBlockingSpanProcessor<S: SpanProcessor> {
    span_processor: S,
}

impl<S: SpanProcessor> NonBlockingSpanProcessor<S> {
    pub fn new(span_processor: S) -> NonBlockingSpanProcessor<S> {
        NonBlockingSpanProcessor { span_processor }
    }
}

impl<S: SpanProcessor> SpanProcessor for NonBlockingSpanProcessor<S> {
    fn on_start(&self, span: &mut Span, cx: &Context) {
        self.span_processor.on_start(span, cx)
    }

    fn on_end(&self, span: SpanData) {
        self.span_processor.on_end(span)
    }

    fn force_flush(&self) -> TraceResult<()> {
        self.span_processor.force_flush()
    }

    /// We only flush instead of shutting down
    fn shutdown(&mut self) -> TraceResult<()> {
        self.force_flush()
    }
}
