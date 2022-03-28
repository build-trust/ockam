pub trait AsyncStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static
{
}

impl AsyncStream for tokio::net::TcpStream {}

impl AsyncStream for tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream> {}
