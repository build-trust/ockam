use std::fmt::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::Event;
use tracing::{span, Subscriber};

#[derive(Clone)]
pub struct LogCollector {
    pub messages: Arc<Mutex<Vec<String>>>,
}

impl LogCollector {
    pub fn contains(&self, s: &str) -> bool {
        let messages = self.messages.lock().unwrap();

        !messages
            .iter()
            .filter(|m| m.contains(s))
            .collect::<Vec<&String>>()
            .is_empty()
    }
}

impl Subscriber for LogCollector {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(0)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut m: String = "".into();
        let mut visitor = StringVisitor { string: &mut m };
        event.record(&mut visitor);
        self.messages.lock().unwrap().push(m);
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}

    fn clone_span(&self, _: &span::Id) -> span::Id {
        span::Id::from_u64(0)
    }
}

impl LogCollector {
    pub fn new() -> LogCollector {
        LogCollector {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn setup() -> LogCollector {
        let collector = LogCollector::new();
        tracing::subscriber::set_global_default(collector.clone()).unwrap();
        collector
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(self.string, "{} = {:?}; ", field.name(), value).unwrap();
    }
}
