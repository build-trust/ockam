use multimap::MultiMap;
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_sdk::export::trace::SpanData;
use std::fmt::{Display, Formatter};
use treeline::Tree;

/// TestSpan holds some of the SpanData retrieved from tracing for testing
/// A TestSpan is uniquely identified by a TestSpanId which is the pair trace id + span id
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSpan {
    id: TestSpanId,
    parent_span_id: SpanId,
    name: String,
    attributes: Vec<(String, String)>,
}

impl Display for TestSpan {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.to_string().as_str())
    }
}

impl TestSpan {
    /// Return the trace id + span id
    pub fn id(&self) -> TestSpanId {
        self.id.clone()
    }

    /// Return the span name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the id of the parent span
    pub fn parent_id(&self) -> TestSpanId {
        TestSpanId {
            trace_id: self.trace_id(),
            span_id: self.parent_span_id,
        }
    }

    /// Return the trace id of this span
    pub fn trace_id(&self) -> TraceId {
        self.id.trace_id
    }

    /// Create a TestSpan from SpanData
    pub fn from_span_data(span_data: &SpanData) -> TestSpan {
        TestSpan {
            id: TestSpanId {
                trace_id: span_data.span_context.trace_id(),
                span_id: span_data.span_context.span_id(),
            },
            parent_span_id: span_data.parent_span_id,
            name: span_data.name.to_string(),
            attributes: span_data
                .attributes
                .iter()
                .map(|kv| (kv.key.to_string(), kv.value.to_string()))
                .collect(),
        }
    }
}

/// Identifier for a TestSpan
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestSpanId {
    trace_id: TraceId,
    span_id: SpanId,
}

impl Display for TestSpanId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{} - {}", self.trace_id, self.span_id))
    }
}

/// Tree of TestSpans forming a trace where the
/// parent / child relationship uses the span.parent_span_id attribute
#[derive(Debug)]
pub struct Trace(pub Tree<TestSpan>);

impl Display for Trace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Trace {
    /// Create a list of traces from exported spans
    /// Note that a list of spans define a list of traces and not just one trace
    /// Because there might be several root spans in the input data
    pub fn from_span_data(spans: Vec<SpanData>) -> Vec<Trace> {
        // sort the spans by parent_span_id
        let mut spans_by_parent_span_id = MultiMap::new();
        for span in &spans {
            let test_span = TestSpan::from_span_data(span);
            spans_by_parent_span_id.insert(test_span.parent_id(), test_span)
        }

        // iterate on the list of all the spans
        // to get the root spans
        let mut test_spans: Vec<TestSpan> = vec![];
        for (_, ts) in spans_by_parent_span_id.iter() {
            test_spans.push(ts.clone());
        }
        let roots = test_spans
            .iter()
            .filter(|ts| ts.parent_span_id == SpanId::from(0));

        // for each root span, create a Trace
        let mut result = vec![];
        for root in roots {
            result.push(Trace(Self::make_trace_tree(
                root.clone(),
                &spans_by_parent_span_id,
            )))
        }
        result
    }

    /// Create a Tree<TestSpan> node from a root by appending its children to the Tree node
    fn make_trace_tree(
        root: TestSpan,
        tree_map: &MultiMap<TestSpanId, TestSpan>,
    ) -> Tree<TestSpan> {
        let mut tree = Tree::root(root.clone());
        if let Some(children) = tree_map.get_vec(&root.id()) {
            for child in children.iter() {
                tree.push(Self::make_trace_tree(child.clone(), tree_map));
            }
        }
        tree
    }
}
