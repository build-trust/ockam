#[cfg(feature = "std")]
use crate::OpenTelemetryContext;
#[cfg(feature = "tracing_context")]
use crate::OCKAM_TRACER_NAME;
use crate::{compat::vec::Vec, Message, Route};
#[cfg(feature = "std")]
use cfg_if::cfg_if;
use core::fmt::{self, Display, Formatter};
#[cfg(feature = "tracing_context")]
use opentelemetry::{
    global,
    trace::{Link, SpanBuilder, TraceContextExt, Tracer},
    Context,
};
use serde::{Deserialize, Serialize};

/// A generic transport message type.
///
/// This type is exposed in `ockam_core` (and the root `ockam` crate) in
/// order to provide a mechanism for third-party developers to create
/// custom transport channel routers.
///
/// Casual users of Ockam should never have to interact with this type
/// directly.
///
/// # Examples
///
/// See `ockam_transport_tcp::workers::sender::TcpSendWorker` for a usage example.
///
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Message)]
pub struct TransportMessage {
    /// The transport protocol version.
    pub version: u8,
    /// Onward message route.
    pub onward_route: Route,
    /// Return message route.
    ///
    /// This field must be populated by routers handling this message
    /// along the way.
    pub return_route: Route,
    /// The message payload.
    pub payload: Vec<u8>,
    /// An optional tracing context
    #[cfg(feature = "tracing_context")]
    pub tracing_context: Option<String>,
}

impl TransportMessage {
    /// Create a new v1 transport message with empty return route.
    pub fn v1(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 1,
            onward_route: onward_route.into(),
            return_route: return_route.into(),
            payload,
            #[cfg(feature = "tracing_context")]
            tracing_context: None,
        }
    }

    /// Return a TransportMessage with a new tracing context:
    ///    - A new trace is started
    ///    - The previous trace and the new trace are linked together
    ///
    /// We start a new trace here in order to make sure that each transport message is always
    /// associated to a globally unique trace id and then cannot be correlated with another transport
    /// message that would leave the same node for example.
    ///
    /// We can still navigate the two created traces as one thanks to their link.
    #[cfg(feature = "std")]
    pub fn start_new_tracing_context(self, _tracing_context: OpenTelemetryContext) -> Self {
        cfg_if! {
            if #[cfg(feature = "tracing_context")] {
                // start a new trace for this transport message, and link it to the previous trace, via the current tracing context
                let tracer = global::tracer(OCKAM_TRACER_NAME);
                let span_builder = SpanBuilder::from_name("TransportMessage::start_trace")
                      .with_links(vec![Link::new(_tracing_context.extract().span().span_context().clone(), vec![])]);
                let span = tracer.build_with_context(span_builder, &Context::default());
                let cx = Context::current_with_span(span);

                // create a span to close the previous trace and link it to the new trace
                let span_builder = SpanBuilder::from_name("TransportMessage::end_trace")
                                 .with_links(vec![Link::new(cx.span().span_context().clone(), vec![])]);
                let _ = tracer.build_with_context(span_builder, &_tracing_context.extract());

                // create the new opentelemetry context
                let tracing_context = OpenTelemetryContext::inject(&cx);

                Self {
                    tracing_context: Some(tracing_context.to_string()),
                    ..self
                }
            } else {
                self
            }
        }
    }

    /// Return the tracing context
    #[cfg(feature = "tracing_context")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        cfg_if! {
            if #[cfg(feature = "tracing_context")] {
                match self.tracing_context.as_ref() {
                    Some(tracing_context) => OpenTelemetryContext::from_remote_context(tracing_context),
                    None => OpenTelemetryContext::current(),
                }
            } else {
                OpenTelemetryContext::current()
            }
        }
    }
}

impl Display for TransportMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Message (onward route: {}, return route: {})",
            self.onward_route, self.return_route
        )
    }
}
