use core::fmt::{Display, Formatter};
use core::str::FromStr;
use ockam_core::errcode::{Kind, Origin};
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing_opentelemetry::OtelData;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Registry;

const TRACE_CONTEXT_PROPAGATION_SPAN: &str = "trace context propagation";

/// Serializable data type to hold the opentelemetry propagation context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenTelemetryContext(HashMap<String, String>);

impl OpenTelemetryContext {
    /// Recover an OpenTelemetry context from the currently serialized data
    pub fn extract(&self) -> opentelemetry::Context {
        global::get_text_map_propagator(|propagator| propagator.extract(self))
    }

    /// Serialize the current OpenTelemetry context as OpenTelemetryContext
    pub fn inject(context: &opentelemetry::Context) -> Self {
        global::get_text_map_propagator(|propagator| {
            let mut propagation_context = OpenTelemetryContext::empty();
            propagator.inject_context(context, &mut propagation_context);
            propagation_context
        })
    }

    /// Return the current OpenTelemetryContext
    pub fn current() -> OpenTelemetryContext {
        // In order to get the current OpenTelemetry context that is connected to the
        // current span, as instrumented with the #[instrument] attribute, we need to:
        //
        //   1. Create a temporary span.
        //   2. Get its data, given its id, from the global registry.
        //   3. In the span extensions we can find the OpenTelemetry context that is used to attribute span ids.
        //      That context contains the span id of the latest span created with OpenTelemetry.
        //      That span is not the dummy span created below but the latest span created with #[instrument] in the
        //      current call stack.
        //      Note that opentelemetry::Context::current() would return a Context which only contains the latest context
        //      created with `tracer::in_span(...)` which is at the root of this trace. This is why we have to dig deep
        //      in order to retrieve the correct span id.
        //   4. Remove the OtelData extension so that our dummy "trace context propagation span" doesn't get emitted.
        let span = tracing::trace_span!(TRACE_CONTEXT_PROPAGATION_SPAN);
        let mut result = None;
        tracing::dispatcher::get_default(|dispatcher| {
            if let Some(registry) = dispatcher.downcast_ref::<Registry>() {
                if let Some(id) = span.id() {
                    if let Some(span) = registry.span(&id) {
                        let mut extensions = span.extensions_mut();
                        if let Some(OtelData {
                            builder: _,
                            parent_cx,
                        }) = extensions.remove::<OtelData>()
                        {
                            result = Some(OpenTelemetryContext::inject(&parent_cx))
                        }
                    }
                }
            };
        });
        // If, for some reason, we cannot retrieve the proper tracing context, we use the latest known
        // OpenTelemetry context
        result.unwrap_or_else(|| OpenTelemetryContext::inject(&opentelemetry::Context::current()))
    }

    fn empty() -> Self {
        Self(HashMap::new())
    }

    /// Return the keys and values for testing
    pub fn as_map(&self) -> HashMap<String, String> {
        self.0.clone()
    }
}

impl Display for OpenTelemetryContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&serde_json::to_string(&self).map_err(|_| core::fmt::Error)?)
    }
}

impl Injector for OpenTelemetryContext {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
}

impl Extractor for OpenTelemetryContext {
    fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_owned();
        self.0.get(&key).map(|v| v.as_ref())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_ref()).collect()
    }
}

/// Parse the OpenTelemetry context from a String
impl TryFrom<&str> for OpenTelemetryContext {
    type Error = ockam_core::Error;

    fn try_from(value: &str) -> ockam_core::Result<Self> {
        opentelemetry_context_parser(value)
    }
}

impl FromStr for OpenTelemetryContext {
    type Err = ockam_core::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

/// Parse the OpenTelemetry context from a String
impl TryFrom<String> for OpenTelemetryContext {
    type Error = ockam_core::Error;

    fn try_from(value: String) -> ockam_core::Result<Self> {
        opentelemetry_context_parser(&value)
    }
}

/// Parse the OpenTelemetry context from a String
pub fn opentelemetry_context_parser(input: &str) -> ockam_core::Result<OpenTelemetryContext> {
    serde_json::from_str(input).map_err(|e| {
        ockam_core::Error::new(
            Origin::Api,
            Kind::Serialization,
            format!("Invalid OpenTelemetry context: {input}. Got error: {e:?}"),
        )
    })
}
