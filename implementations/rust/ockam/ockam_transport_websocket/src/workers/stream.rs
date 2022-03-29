/// A trait alias to define an AsyncStream returned
/// when creating or accepting WebSocket connections.
///
/// This is used to reduce the complexity of the definition
/// of the structs that use WebSocket streams.
pub(crate) trait AsyncStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static
{
}

impl AsyncStream for tokio::net::TcpStream {}

impl AsyncStream for tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream> {}
