//! Trait definition and stub and external crates implementations needed to use the core transport functionality.
//!
//! These traits have to be implemented by a TCP(Could be generalized to other protocols) implementation in order to use the utilities, such as the [workers](crate::tcp::workers) modules, provided by this crate to implement transport protocols.
//!
//! The most important parts of this module are the [IntoSplit], [TcpAccepter] and [TcpBinder] traits, these generalize the behavior we use from a Tcp stream or socket from the standard library so that we can implement in other libraries(such as smoltcp).
//!
//! The rest of the module are no-op implementations and implementations for `tokio::net`.
use self::io::{AsyncRead, AsyncWrite};

use crate::TransportError;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;

pub mod io;

/// A channel that implements `IntoSplit` can be split into two owned halves: one for reading and one for writing
///
/// Both halves should be able to be used separately and moved independently.
///
/// This trait generalizes [tokio::net::TcpStream::into_split].
///
/// # Example
/// ```no_run
/// use tokio::net::TcpStream;
/// use ockam_transport_core::tcp::traits::IntoSplit;
/// # #[tokio::main]
/// # async fn main() {
/// let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
/// let (read_half, write_half) = stream.into_split();
/// # }
/// ```
pub trait IntoSplit {
    /// Type of the read half of the splittable channel.
    type ReadHalf: AsyncRead + 'static;

    /// Type of the write half of the splittable channel.
    type WriteHalf: AsyncWrite + 'static;

    /// Consumes the splittable channel and returns both halfs as a pair `(ReadHalf, WriteHalf)`
    fn into_split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

// Implementation of `IntoSplit` for tokio's TcpStream, it simply forwards the method.
#[cfg(feature = "std")]
impl IntoSplit for tokio::net::TcpStream {
    type ReadHalf = tokio::net::tcp::OwnedReadHalf;
    type WriteHalf = tokio::net::tcp::OwnedWriteHalf;

    fn into_split(self) -> (Self::ReadHalf, Self::WriteHalf) {
        tokio::net::TcpStream::into_split(self)
    }
}

/// A `TcpStreamConnector` will connect to a given endpoint and return a stream.
///
/// This traits generalizes `tokio::net::TcpStream::connect` to use it with non-std implementations of the TCP stack.
///
/// <strong>Note:</strong> that `connect` isn't static in this version because for smoltcp we need a `StackFacade` instance(For type security reasons) in order to connect.
/// # Example
/// ## Using tokio's socket implementation connects to 127.0.0.1:8080 send "hello" and await for 8 bytes to be returned.
/// ```no_run
/// use ockam_transport_core::tcp::traits::{TcpStreamConnector, TokioTcpConnector};
/// use ockam_transport_core::tcp::traits::io::{AsyncReadExt, AsyncWriteExt};
/// # #[tokio::main]
/// # async fn main() {
/// // Connect using tokio's socket
/// let connector = TokioTcpConnector;
/// let stream = connector.connect("127.0.0.1:8080").await.unwrap();
///
/// // Split the created stream into recv and send halves
/// let (mut rx, mut tx) = stream.into_split();
///
/// // Send "hello"
/// let send_buf = "hello".as_bytes();
/// tx.write_all(&send_buf).await.unwrap();
///
/// // Wait to recieve a 8 bytes
/// let mut recv_buf = [0u8; 8];
/// rx.read_exact(&mut recv_buf).await.unwrap();
/// # }
/// ```
#[async_trait]
pub trait TcpStreamConnector<A> {
    /// Type of the returned stream.
    type Stream: IntoSplit;

    /// Creates an stream to the given endpoint.
    async fn connect(&self, peer: A) -> Result<Self::Stream, TransportError>;
}

/// No-op implementor of TcpStreamConnector.
///
/// This is currently only userd by [TcpListenProcessor](crate::tcp::workers::TcpListenProcessor) so that we can use [TcpSendWorker](crate::tcp::workers::TcpSendWorker) without having to receive a connector that won't be used since we always pass the already connected stream.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoConnector<T>(pub core::marker::PhantomData<T>);

// No-op implementation.
#[async_trait]
impl<A, B> TcpStreamConnector<A> for NoConnector<B>
where
    A: Send + 'static,
    B: IntoSplit + Sync,
{
    type Stream = B;
    async fn connect(&self, _: A) -> Result<Self::Stream, TransportError> {
        unimplemented!()
    }
}

/// A `TcpBinder` will be able to bind to a specific address and yield a listener that will be ready to accept new connections.
///
/// This generalizes `tokio::net::TcpListener::bind` to use it with non-std implementations of the TCP stack.
///
/// <strong>Note:</strong> that `bind` isn't static in this version because for smoltcp we need a `StackFacade` instance(For type security reasons) in order to connect.
/// # Example
/// ## Using tokio's socket bind it to 127.0.0.1:8080 and await for new connections and echo 8 bytes.
/// ```no_run
/// use ockam_transport_core::tcp::traits::{TcpBinder, TokioTcpBinder};
/// use ockam_transport_core::tcp::traits::io::{AsyncReadExt, AsyncWriteExt};
/// # #[tokio::main]
/// # async fn main() {
/// // Bind to 127.0.0.1:8080
/// let binder = TokioTcpBinder;
/// let listener = binder.bind("127.0.0.1:8080").await.unwrap();
///
/// // Await for a new connection
/// let (stream, peer) = listener.accept().await.unwrap();
///
/// // Split the stream
/// let (mut rx, mut tx) = stream.into_split();
///
/// let mut buf = [0u8;8];
///
/// // Read 8 bytes
/// rx.read_exact(&mut buf).await.unwrap();
///
/// // Write back the read 8 bytes
/// tx.write_all(&buf).await.unwrap();
/// # }
/// ```
#[async_trait]
pub trait TcpBinder<A> {
    /// Type of the returned listener.
    type Listener: TcpAccepter;

    /// Binds to address `binding` and on sucess returns a `TcpAccepter` that can be used to accept new TCP connections.
    /// <strong>Note:</note> The user can't depend that binding to port 0 works since there is no insurance that in an non-std platform there is a way to get an available port.
    async fn bind(&self, binding: A) -> Result<Self::Listener, TransportError>;
}

/// A TcpAccepter can await for new connections and return an stream that can be used as a channel with the connection
///
/// This generalizes the behavior of `tokio::net::TcpListener::accept` to use it with non-std implementation of the TCP stack.
/// # Example
/// ## Using tokio's socket bind it to 127.0.0.1:8080 and await for new connections and echo 8 bytes.
/// ```no_run
/// use ockam_transport_core::tcp::traits::{TcpBinder, TokioTcpBinder};
/// use ockam_transport_core::tcp::traits::io::{AsyncReadExt, AsyncWriteExt};
/// # #[tokio::main]
/// # async fn main() {
/// // Bind to 127.0.0.1:8080
/// let binder = TokioTcpBinder;
/// let listener = binder.bind("127.0.0.1:8080").await.unwrap();
///
/// // Await for a new connection
/// let (stream, peer) = listener.accept().await.unwrap();
///
/// // Split the stream
/// let (mut rx, mut tx) = stream.into_split();
///
/// let mut buf = [0u8;8];
///
/// // Read 8 bytes
/// rx.read_exact(&mut buf).await.unwrap();
///
/// // Write back the read 8 bytes
/// tx.write_all(&buf).await.unwrap();
/// # }
/// ```
#[async_trait]
pub trait TcpAccepter {
    /// Type of the returned stream.
    type Stream: IntoSplit;

    /// Type for the address of the connected peer.
    type Peer;

    /// Awaits for an incoming connection and on sucess returns a tuple (stream, peer).
    async fn accept(&mut self) -> Result<(Self::Stream, Self::Peer), TransportError>;
}

/// An `EndpointResolver` can resolve a string and returns the parsed TCP address and hostnames if there are any.
pub trait EndpointResolver<T = ()> {
    /// Type of the hostnames (Note: this is a set of multiple hostnames).
    type Hostnames;
    /// Type of the returned address.
    type Peer;
    /// Takes a `&str` and resolves the address and hostnames, if any, returns an error in case resolution is impossible.
    fn resolve_endpoint(peer: &str) -> Result<(Self::Peer, Self::Hostnames), ockam_core::Error>;
}

// Implementation of `TcpAccepter` for tokio
#[cfg(feature = "std")]
#[async_trait]
impl TcpAccepter for tokio::net::TcpListener {
    type Stream = tokio::net::TcpStream;
    type Peer = std::net::SocketAddr;
    async fn accept(&mut self) -> Result<(Self::Stream, Self::Peer), TransportError> {
        tokio::net::TcpListener::accept(self)
            .await
            .map_err(TransportError::from)
    }
}

/// Type that can be instanced that implements [TcpBinder] for tokio's sockets.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub struct TokioTcpBinder;

#[cfg(feature = "std")]
#[async_trait]
impl<A> TcpBinder<A> for TokioTcpBinder
where
    A: tokio::net::ToSocketAddrs + Send + 'static,
{
    type Listener = tokio::net::TcpListener;
    async fn bind(&self, addr: A) -> Result<Self::Listener, TransportError> {
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(TransportError::from)
    }
}

/// Type that can be instanced that implements [TcpStreamConnector] for tokio's sockets.
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub struct TokioTcpConnector;

#[cfg(feature = "std")]
#[async_trait]
impl<A> TcpStreamConnector<A> for TokioTcpConnector
where
    A: Send + tokio::net::ToSocketAddrs + 'static,
{
    type Stream = tokio::net::TcpStream;

    async fn connect(&self, peer: A) -> Result<Self::Stream, TransportError> {
        tokio::net::TcpStream::connect(peer)
            .await
            .map_err(TransportError::from)
    }
}
