use chrono::{DateTime, Utc};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::{Context, Key, KeyValue};

/// This struct is used as a convenient way to set attributes
/// on the current span
pub struct CurrentSpan;

impl CurrentSpan {
    /// Set an attribute (key/value) on the current OpenTelemetry span
    pub fn set_attribute(name: &Key, value: &str) {
        Context::current()
            .span()
            .set_attribute(KeyValue::new(name.clone(), value.to_string()));
    }

    /// Set the current time on the current OpenTelemetry span
    /// This function makes sure that we use the same datetime format on time attributes
    pub fn set_attribute_time(name: &Key) {
        let current_utc: DateTime<Utc> = Utc::now();
        let formatted_time: String = current_utc.format("%Y-%m-%dT%H:%M:%S.%3f").to_string();
        CurrentSpan::set_attribute(name, &formatted_time)
    }
}
