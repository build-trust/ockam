use crate::alloc::string::ToString;
use crate::compat::string::String;
use crate::errcode::{Kind, Origin};
#[cfg(feature = "std")]
use crate::OpenTelemetryContext;
#[cfg(feature = "std")]
use crate::OCKAM_TRACER_NAME;
use crate::{compat::vec::Vec, Decodable, Encodable, Encoded, Message, Route};
use crate::{Error, Result};
use core::fmt::{self, Display, Formatter};
#[cfg(feature = "std")]
use opentelemetry::{
    global,
    trace::{Link, SpanBuilder, TraceContextExt, Tracer},
    Context,
};

/// Version for transport messages
pub type ProtocolVersion = u8;

/// Latest protocol version for transport messages
pub const LATEST_PROTOCOL_VERSION: ProtocolVersion = 2;

/// Protocol version for transport messages. This version doesn't have a tracing_context field
pub const PROTOCOL_VERSION_V1: ProtocolVersion = 1;

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
#[derive(Debug, Clone, Eq, PartialEq, Message)]
// TODO: This should be deleted in favor of transport-specific structures
//  defined at transport crates
pub struct TransportMessage {
    /// The transport protocol version.
    pub version: ProtocolVersion,
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
    pub tracing_context: Option<String>,
}

impl TransportMessage {
    /// Create the latest version of a transport message with an empty return route.
    pub fn latest(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        TransportMessage::new(
            LATEST_PROTOCOL_VERSION,
            onward_route.into(),
            return_route.into(),
            payload,
            None,
        )
    }

    /// Create a transport message in version v1
    pub fn v1(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        TransportMessage::new(
            PROTOCOL_VERSION_V1,
            onward_route,
            return_route,
            payload,
            None,
        )
    }

    /// Create a new transport message
    pub fn new(
        version: ProtocolVersion,
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
        tracing_context: Option<String>,
    ) -> Self {
        Self {
            version,
            onward_route: onward_route.into(),
            return_route: return_route.into(),
            payload,
            tracing_context,
        }
    }

    /// Decode the transport message according to the first byte, which is the version number
    pub fn decode_message(buf: Vec<u8>) -> Result<TransportMessage> {
        if buf.is_empty() {
            return Err(Error::new(
                Origin::Transport,
                Kind::Serialization,
                "empty buffer, no transport message received".to_string(),
            ));
        };
        let version = buf[0];
        match version {
            PROTOCOL_VERSION_V1 => TransportMessageV1::decode(&buf)
                .map(|t| t.to_latest())
                .map_err(|e| {
                    Error::new(
                        Origin::Transport,
                        Kind::Serialization,
                        format!("Error decoding message: {:?}", e),
                    )
                }),
            LATEST_PROTOCOL_VERSION => TransportMessage::decode(&buf).map_err(|e| {
                Error::new(
                    Origin::Transport,
                    Kind::Serialization,
                    format!("Error decoding message: {:?}", e),
                )
            }),
            v => Err(Error::new(
                Origin::Transport,
                Kind::Serialization,
                format!("Unsupported version: {v}"),
            )),
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
        // start a new trace for this transport message, and link it to the previous trace, via the current tracing context
        let tracer = global::tracer(OCKAM_TRACER_NAME);
        let span_builder =
            SpanBuilder::from_name("TransportMessage::start_trace").with_links(vec![Link::new(
                _tracing_context.extract().span().span_context().clone(),
                vec![],
                0,
            )]);
        let span = tracer.build_with_context(span_builder, &Context::default());
        let cx = Context::current_with_span(span);

        // create a span to close the previous trace and link it to the new trace
        let span_builder = SpanBuilder::from_name("TransportMessage::end_trace")
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

impl Display for TransportMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Message (onward route: {}, return route: {})",
            self.onward_route, self.return_route
        )
    }
}

impl Encodable for TransportMessage {
    fn encode(self) -> Result<Encoded> {
        let tracing = if let Some(tracing_context) = self.tracing_context.as_ref() {
            1 + crate::bare::size_of_slice(tracing_context.as_bytes())
        } else {
            1
        };

        let mut encoded = Vec::with_capacity(
            1 + self.onward_route.encoded_size()
                + self.return_route.encoded_size()
                + crate::bare::size_of_slice(&self.payload)
                + tracing,
        );
        encoded.push(self.version);
        self.onward_route.manual_encode(&mut encoded);
        self.return_route.manual_encode(&mut encoded);
        crate::bare::write_slice(&mut encoded, &self.payload);
        if let Some(tracing_context) = self.tracing_context.as_ref() {
            encoded.push(1);
            crate::bare::write_str(&mut encoded, tracing_context);
        } else {
            encoded.push(0);
        }
        Ok(encoded)
    }
}

impl Decodable for TransportMessage {
    fn decode(slice: &[u8]) -> Result<Self> {
        Self::internal_decode(slice).ok_or_else(|| {
            Error::new(
                Origin::Transport,
                Kind::Protocol,
                "Failed to decode TransportMessage",
            )
        })
    }
}

impl TransportMessage {
    fn internal_decode(slice: &[u8]) -> Option<Self> {
        let mut index = 0;
        let version = slice.get(index)?;
        index += 1;

        let onward_route = Route::manual_decode(slice, &mut index)?;
        let return_route = Route::manual_decode(slice, &mut index)?;
        let payload = crate::bare::read_slice(slice, &mut index)?;

        let present = slice.get(index).unwrap_or(&0);
        index += 1;
        let tracing_context = if present == &1 {
            crate::bare::read_str(slice, &mut index).map(|s| s.to_string())
        } else {
            None
        };

        Some(Self {
            version: *version,
            onward_route,
            return_route,
            payload: payload.to_vec(),
            tracing_context,
        })
    }
}

/// This is version 1 of the transport message without a tracing_context field
#[derive(Debug, Clone, Eq, PartialEq, Message)]
pub struct TransportMessageV1 {
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
}

impl TransportMessageV1 {
    /// Convert a transport message v1 to the latest version of the protocol
    pub fn to_latest(self) -> TransportMessage {
        TransportMessage {
            version: PROTOCOL_VERSION_V1,
            onward_route: self.onward_route,
            return_route: self.return_route,
            payload: self.payload,
            tracing_context: None,
        }
    }

    /// Create a new transport message with version v1
    pub fn new(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 1,
            onward_route: onward_route.into(),
            return_route: return_route.into(),
            payload,
        }
    }
}

impl Encodable for TransportMessageV1 {
    fn encode(self) -> Result<Encoded> {
        let mut encoded = Vec::with_capacity(
            1 + self.onward_route.encoded_size()
                + self.return_route.encoded_size()
                + crate::bare::size_of_slice(&self.payload),
        );
        encoded.push(self.version);
        self.onward_route.manual_encode(&mut encoded);
        self.return_route.manual_encode(&mut encoded);
        crate::bare::write_slice(&mut encoded, &self.payload);
        encoded.push(0);
        Ok(encoded)
    }
}

impl Decodable for TransportMessageV1 {
    fn decode(slice: &[u8]) -> Result<Self> {
        Self::internal_decode(slice).ok_or_else(|| {
            Error::new(
                Origin::Transport,
                Kind::Protocol,
                "Failed to decode TransportMessage",
            )
        })
    }
}

impl TransportMessageV1 {
    fn internal_decode(slice: &[u8]) -> Option<Self> {
        let mut index = 0;
        let version = slice.get(index)?;
        index += 1;

        let onward_route = Route::manual_decode(slice, &mut index)?;
        let return_route = Route::manual_decode(slice, &mut index)?;
        let payload = crate::bare::read_slice(slice, &mut index)?;

        Some(Self {
            version: *version,
            onward_route,
            return_route,
            payload: payload.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{route, Encodable, TransportMessageV1};

    #[test]
    fn test_encode_decode() {
        let transport_message_v1 =
            TransportMessageV1::new(route!["onward"], route!["return"], vec![]);
        let transport_message_v2 =
            TransportMessage::latest(route!["onward"], route!["return"], vec![]);

        // a v1 message should be decodable as the latest structure
        let encoded_v1 = transport_message_v1.encode().unwrap();
        let expected = TransportMessage::new(
            PROTOCOL_VERSION_V1,
            route!["onward"],
            route!["return"],
            vec![],
            None,
        );
        assert_eq!(
            TransportMessage::decode_message(encoded_v1).unwrap(),
            expected
        );

        // a v2 message should be decodable as the latest version
        let encoded_v2 = transport_message_v2.clone().encode().unwrap();
        assert_eq!(
            TransportMessage::decode_message(encoded_v2).unwrap(),
            transport_message_v2
        );

        // any other version must fail to be decoded
        let encoded_v3 = TransportMessage {
            version: 3,
            onward_route: route![],
            return_route: route![],
            payload: vec![],
            tracing_context: None,
        }
        .encode()
        .unwrap();
        assert!(TransportMessage::decode_message(encoded_v3).is_err());
    }
}
