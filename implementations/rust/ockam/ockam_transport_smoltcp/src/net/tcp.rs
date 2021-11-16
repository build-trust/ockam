use core::pin;
use ockam_core::async_trait;
use ockam_core::compat::task::{Context, Poll};
use ockam_transport_core::tcp::traits::io::Result;
use ockam_transport_core::tcp::traits::io::{AsyncRead, AsyncWrite};
use ockam_transport_core::tcp::traits::IntoSplit;
use smoltcp::iface::SocketHandle;
use smoltcp::socket::TcpState;
use smoltcp::wire::IpEndpoint;

use super::StackFacade;

#[derive(Debug)]
pub(crate) struct AsyncTcpSocket {
    socket_handle: SocketHandle,
    stack: StackFacade,
}

impl Drop for AsyncTcpSocket {
    fn drop(&mut self) {
        self.close();
    }
}

use ockam_core::compat::sync::Arc;

#[derive(Debug, Clone)]
pub struct AsyncTcpStream {
    // TODO allocation
    // We need shared ownership of the AsyncTcpSocket since Dropping the `AsyncTcpSocket` should close it.
    // Since ockam_core still needs `alloc` we are using `Arc`. When we go to an alloc-less version we can either:
    // * AsyncTcpSocket drop doesn't remove it/close it, instead the creator must make sure that it removes it after using.
    // * Use heapless's Arc
    inner: Arc<AsyncTcpSocket>,
}

#[async_trait]
impl IntoSplit for AsyncTcpStream {
    type ReadHalf = AsyncTcpStream;

    type WriteHalf = AsyncTcpStream;

    fn into_split(self) -> (Self::ReadHalf, Self::WriteHalf) {
        (self.clone(), self)
    }
}

impl AsyncTcpSocket {
    pub(crate) fn new(socket_handle: SocketHandle, stack: StackFacade) -> Self {
        Self {
            socket_handle,
            stack,
        }
    }

    pub(crate) fn remote_endpoint(&self) -> IpEndpoint {
        self.stack
            .with_handle(self.socket_handle, |s, _| s.remote_endpoint())
    }

    fn get_state(&self) -> TcpState {
        self.stack.with_handle(self.socket_handle, |s, _| s.state())
    }

    fn register_send_waker(&self, cx: &mut Context) {
        self.stack
            .with_handle(self.socket_handle, |s, _| s.register_recv_waker(cx.waker()));
    }

    fn register_recv_waker(&self, cx: &mut Context) {
        self.stack
            .with_handle(self.socket_handle, |s, _| s.register_recv_waker(cx.waker()));
    }

    fn close(&self) {
        self.stack.with_handle(self.socket_handle, |s, _| s.close());
    }

    pub(crate) fn get_connection_status(&mut self) -> (bool, IpEndpoint) {
        self.stack.with_handle(self.socket_handle, |s, _| {
            (s.is_active(), s.remote_endpoint())
        })
    }

    pub(crate) async fn connect<T, U>(
        &mut self,
        remote_endpoint: T,
        local_endpoint: U,
    ) -> smoltcp::Result<()>
    where
        T: Into<IpEndpoint>,
        U: Into<IpEndpoint>,
    {
        self.stack.with_handle(self.socket_handle, |s, cx| {
            s.connect(cx, remote_endpoint, local_endpoint)
        })?;

        futures::future::poll_fn(|cx| match self.get_state() {
            TcpState::Closed | TcpState::TimeWait => {
                Poll::Ready(Err(smoltcp::Error::Unaddressable))
            }
            TcpState::Listen => Poll::Ready(Err(smoltcp::Error::Illegal)),
            TcpState::SynSent | TcpState::SynReceived => {
                self.register_send_waker(cx);
                Poll::Pending
            }
            _ => Poll::Ready(Ok(())),
        })
        .await
    }

    pub(crate) async fn listen<T>(&mut self, local_endpoint: T) -> smoltcp::Result<()>
    where
        T: Into<IpEndpoint>,
    {
        self.stack.with_handle(self.socket_handle, |socket, _| {
            socket.listen(local_endpoint)
        })?;

        futures::future::poll_fn(|cx| match self.get_state() {
            TcpState::Closed | TcpState::TimeWait => {
                Poll::Ready(Err(smoltcp::Error::Unaddressable))
            }
            TcpState::Listen => Poll::Ready(Ok(())),
            TcpState::SynSent | TcpState::SynReceived => {
                self.register_send_waker(cx);
                Poll::Pending
            }
            _ => Poll::Ready(Ok(())),
        })
        .await
    }

    pub(super) async fn accept(mut self) -> (AsyncTcpStream, IpEndpoint) {
        futures::future::poll_fn(|cx| {
            if self.get_connection_status().0 {
                Poll::Ready(())
            } else {
                self.register_recv_waker(cx);
                Poll::Pending
            }
        })
        .await;

        let remote_endpoint = self.remote_endpoint();
        let stream = AsyncTcpStream {
            inner: Arc::new(self),
        };
        (stream, remote_endpoint)
    }

    pub(crate) fn into_stream(self) -> AsyncTcpStream {
        AsyncTcpStream {
            inner: Arc::new(self),
        }
    }
}

// We could implement AsyncBufRead also but with the current protocol we always know how many bytes
// we want and we can allocate that and pass to the reader.
impl AsyncRead for AsyncTcpStream {
    fn poll_read(
        self: pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let recv_slice = self
            .inner
            .stack
            .with_handle(self.inner.socket_handle, |s, _| s.recv_slice(buf));
        match recv_slice {
            // If the buffer is empty recv_slice will always return 0
            Ok(0) if !buf.is_empty() => {
                self.inner.register_recv_waker(cx);
                Poll::Pending
            }
            Ok(n) => Poll::Ready(Ok(n)),
            Err(smoltcp::Error::Finished) => Poll::Ready(Ok(0)),
            res => Poll::Ready(Ok(res?)),
        }
    }
}

impl AsyncWrite for AsyncTcpStream {
    fn poll_write(
        self: pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        let send_slice = self
            .inner
            .stack
            .with_handle(self.inner.socket_handle, |s, _| s.send_slice(buf));
        match send_slice {
            Ok(0) => {
                self.inner.register_send_waker(cx);
                Poll::Pending
            }
            Ok(n) => Poll::Ready(Ok(n)),
            res => Poll::Ready(Ok(res?)),
        }
    }
}
