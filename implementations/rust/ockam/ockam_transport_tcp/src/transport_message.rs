use cfg_if::cfg_if;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::string::{String, ToString};
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
#[cfg(feature = "std")]
use ockam_core::OCKAM_TRACER_NAME;
use ockam_core::{CowBytes, LocalMessage, Route};
#[cfg(feature = "std")]
use opentelemetry::{
    global,
    trace::{Link, SpanBuilder, TraceContextExt, Tracer},
    Context,
};

/// TCP transport message type.
#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct TcpTransportMessage<'a> {
    #[n(0)] pub onward_route: Route,
    #[n(1)] pub return_route: Route,
    #[b(2)] pub payload: CowBytes<'a>,
    #[n(3)] pub tracing_context: Option<String>,
}

impl<'a> TcpTransportMessage<'a> {
    /// Constructor.
    pub fn new(
        onward_route: Route,
        return_route: Route,
        payload: CowBytes<'a>,
        tracing_context: Option<String>,
    ) -> Self {
        Self {
            onward_route,
            return_route,
            payload,
            tracing_context,
        }
    }

    /// Return a TcpTransportMessage with a new tracing context:
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
        // start a new trace for this transport message, and link it to the previous trace, via the current tracing context
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let span_builder =
            SpanBuilder::from_name("TcpTransportMessage::start_trace").with_links(vec![Link::new(
                _tracing_context.extract().span().span_context().clone(),
                vec![],
                0,
            )]);
        let span = tracer.build_with_context(span_builder, &Context::default());
        let cx = Context::current_with_span(span);

        // create a span to close the previous trace and link it to the new trace
        let span_builder = SpanBuilder::from_name("TcpTransportMessage::end_trace")
            .with_links(vec![Link::new(cx.span().span_context().clone(), vec![], 0)]);
        let _ = tracer.build_with_context(span_builder, &_tracing_context.extract());

        // create the new opentelemetry context
        let tracing_context = OpenTelemetryContext::inject(&cx);

        Self {
            tracing_context: Some(tracing_context.to_string()),
            ..self
        }
    }

    /// Return the tracing context
    #[cfg(feature = "std")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        match self.tracing_context.as_ref() {
            Some(tracing_context) => OpenTelemetryContext::from_remote_context(tracing_context),
            None => OpenTelemetryContext::current(),
        }
    }
}

impl From<TcpTransportMessage<'_>> for LocalMessage {
    fn from(value: TcpTransportMessage) -> Self {
        let local_message = LocalMessage::new();

        #[cfg(feature = "std")]
        let local_message = local_message.with_tracing_context(value.tracing_context());

        local_message
            .with_onward_route(value.onward_route)
            .with_return_route(value.return_route)
            .with_payload(value.payload.into_owned())
    }
}

impl From<LocalMessage> for TcpTransportMessage<'_> {
    fn from(value: LocalMessage) -> Self {
        let transport_message = Self::new(
            value.onward_route,
            value.return_route,
            CowBytes::from(value.payload),
            None,
        );

        cfg_if! {
            if #[cfg(feature = "std")] {
                // make sure to pass the latest tracing context
                transport_message.start_new_tracing_context(value.tracing_context.update())
            } else {
                transport_message
            }
        }
    }
}
