use core::fmt::{Display, Formatter};
use core::str::FromStr;
use ockam_core::errcode::{Kind, Origin};
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        OpenTelemetryContext::inject(&opentelemetry::Context::current())
    }

    fn empty() -> Self {
        Self(HashMap::new())
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
