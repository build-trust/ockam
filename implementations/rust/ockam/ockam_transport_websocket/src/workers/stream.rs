/// Type alias for `tokio_tungstenite::WebSocketStream`.
pub(crate) type WebSocketStream<S> = tokio_tungstenite::WebSocketStream<S>;

/// Stream created when a server accepts a new connection.
pub(crate) type TcpServerStream = tokio::net::TcpStream;

/// Stream created when a client connects to a server.
pub(crate) type TcpClientStream = tokio_tungstenite::MaybeTlsStream<TcpServerStream>;

/// Trait alias to define an AsyncStream returned
/// when creating or accepting WebSocket connections.
///
/// This is used to reduce the complexity of the definition
/// of the structs that use WebSocket streams.
pub(crate) trait AsyncStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static
{
}

impl AsyncStream for TcpClientStream {}

impl AsyncStream for TcpServerStream {}
