use cfg_if::cfg_if;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::string::String;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{CowBytes, LocalMessage, Route};

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

    /// Specify the tracing context
    #[cfg(feature = "std")]
    pub fn with_tracing_context(self, tracing_context: String) -> Self {
        Self {
            tracing_context: Some(tracing_context),
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
                let new_tracing_context = LocalMessage::start_new_tracing_context(value.tracing_context.update(), "TcpTransportMessage");
                transport_message.with_tracing_context(new_tracing_context)
            } else {
                transport_message
            }
        }
    }
}
