//! Traits based in the `futures::io` crate to support `no_std` compilation.
//!
//! The intend use of these traits is to be implemented for the transport layer to use along the [workers][crate::tcp::workers] in this crate. To see a more complex example of this take a look at the implementation in `ockam_transport_smoltcp::net::tcp` module.
//!
//! The biggest change is using [TransportError] instead of [std::io::Error] for compatibility without std.
//! This module define and implements both [AsyncRead] and [AsyncWrite] from the futures crate but it only implements `write_all` and `read_exact` from [AsyncWriteExt] and [AsyncReadExt] respectively, since those are the only method we use.
//!
//! <strong>Note:</strong> Although the implementation is based on `futures::io` the traits are auto-implemented only for `tokio::io::AsyncRead` and `tokio::io::AsyncWrite` not for the same structs in `futures::io`.
//! This is becaused to play along with the other ockam crates we needed it to be implemented for `tokio` but the implementation from futures was slightly simpler. The differences on implementation are subtle and we might change it in the future to be more similar to `tokio`.
use crate::error::TransportError;
use core::fmt::Debug;
use core::mem;
use core::pin::Pin;
use futures::{ready, Future};
use ockam_core::compat::task::{Context, Poll};

/// Result from IO operations.
pub type Result<T> = core::result::Result<T, TransportError>;

/// Custom definition of `futures::io::AsyncRead` that returns our [Result].
///
/// Reads bytes asyncronously from a source.
pub trait AsyncRead {
    /// Attempt to read from [AsyncRead] into `buf`.
    ///
    /// On success, returns `Poll::Ready(Ok(num_bytes_read))`.
    ///
    /// If no data is available for reading, the method return `Poll::Pending` and arranges for the current task (via `cx.waker().wake_by_ref()`)
    /// to receive a notification when the object becomes readable or is closed.
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8])
        -> Poll<Result<usize>>;
}

//  Auto implementation of `AsyncRead` for any struct implementing `tokio::io::AsyncRead`.
#[cfg(feature = "std")]
impl<T: tokio::io::AsyncRead> AsyncRead for T {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let mut buf = tokio::io::ReadBuf::new(buf);
        let res = tokio::io::AsyncRead::poll_read(self, cx, &mut buf);
        // Tokio's AsyncRead doesn't return the number of bytes read instead this information is stored in `ReadBuf` so here we do the mapping.
        res.map_ok(|()| buf.filled().len())
            .map_err(TransportError::from)
    }
}

/// Custom definition of `futures::io::AsyncWrite` that returns our [Result].
///
/// Write bytes asynchronously.
pub trait AsyncWrite {
    /// Attempt to write bytes from `buf` into the object.
    ///
    /// On sucess, returns `Poll::Ready(Ok(num_bytes_written))`.
    ///
    /// If the object is not ready for writing, the method returns `Poll::Pending` and arranges for the current task (via `cx.waker().wake_by_ref()`) to receive a notification when the object becomes writable or is closed.
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
}

// Auto implementation of AsyncWrite for any trait implementing `tokio::io::AsyncWrite`.
#[cfg(feature = "std")]
impl<T: tokio::io::AsyncWrite> AsyncWrite for T {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        tokio::io::AsyncWrite::poll_write(self, cx, buf).map_err(|err| err.into())
    }
}

/// Custom definition of `futures::io::AsyncWriteExt` compatible with our [AsyncWrite].
///
/// Provides extension methods to [AsyncWrite].
pub trait AsyncWriteExt: AsyncWrite {
    /// Write data into this object.
    ///
    /// Creates a future that will write the entire contes of the buffer `buf` into `AsyncWrite`.
    ///
    /// The returned future will not complete until all the data has been written.
    ///
    /// # Examples
    /// ```
    /// use ockam_transport_core::tcp::traits::io::AsyncWriteExt;
    /// use std::io::Cursor;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut writer = Cursor::new(vec![0u8;5]);
    ///
    /// writer.write_all(&[1, 2, 3, 4]).await.unwrap();
    ///
    /// assert_eq!(writer.into_inner(), [1, 2, 3, 4, 0]);
    /// # }
    /// ```
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self> {
        WriteAll::new(self, buf)
    }
}

/// Custom definition of `futures::io::AsynReadExt` compatible with our [AsyncWrite].
///
/// Provides extension methods for `AsyncRead`.
pub trait AsyncReadExt: AsyncRead {
    /// Creates a future which will read exactly enough bytes to fill `buf`, returning an error ([TransportError::UnexpectedEof]) if end of file is hit sooner.
    ///
    /// The returned future will resolve once the read operation is completed an will keep returning `Poll::Pending` until the the buffer is filled.
    ///
    /// In the case of an error the buffer and the object will be discarded, with the error yielded(This will depend on the mapping between the io error and [TransportError]).
    ///
    /// # Examples
    /// ```
    /// use ockam_transport_core::tcp::traits::io::AsyncReadExt;
    /// use std::io::Cursor;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut reader = Cursor::new([1,2, 3, 4]);
    /// let mut output = [0u8; 4];
    ///
    /// reader.read_exact(&mut output).await.unwrap();
    ///
    /// assert_eq!(output, [1, 2, 3, 4]);
    /// # }
    /// ```
    ///
    /// # Eof is hit before `buf` is filled
    /// ```
    /// use ockam_transport_core::tcp::traits::io::AsyncReadExt;
    /// use ockam_transport_core::TransportError;
    /// use std::io::Cursor;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut reader = Cursor::new([1,2, 3, 4]);
    /// let mut output = [0u8; 5];
    ///
    /// let res = reader.read_exact(&mut output).await;
    ///
    /// assert_eq!(res.unwrap_err(), TransportError::UnexpectedEof);
    /// # }
    /// ```
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExact<'a, Self> {
        ReadExact::new(self, buf)
    }
}

impl<T> AsyncWriteExt for T where T: AsyncWrite {}
impl<T> AsyncReadExt for T where T: AsyncRead {}

/// Future for the [`write_all`](AsyncWriteExt::write_all) method.
#[derive(Debug)]
pub struct WriteAll<'a, W: ?Sized> {
    writer: &'a mut W,
    buf: &'a [u8],
}

impl<W: ?Sized + Unpin> Unpin for WriteAll<'_, W> {}

impl<'a, W: AsyncWrite + ?Sized> WriteAll<'a, W> {
    fn new(writer: &'a mut W, buf: &'a [u8]) -> Self {
        Self { writer, buf }
    }
}

/// Future for the [`read_exact`](AsyncReadExt::read_exact) method.
#[derive(Debug)]
pub struct ReadExact<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [u8],
}

impl<R: ?Sized + Unpin> Unpin for ReadExact<'_, R> {}

impl<'a, R: AsyncRead + ?Sized> ReadExact<'a, R> {
    fn new(reader: &'a mut R, buf: &'a mut [u8]) -> Self {
        Self { reader, buf }
    }
}

impl<'a, W: AsyncWrite + ?Sized + Unpin> Future for WriteAll<'a, W> {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        // Will consume the buffer contents and write them into the writer until it's empty
        while !this.buf.is_empty() {
            // Write into the buffer or returns Pending if it's not readdy
            let n = ready!(Pin::new(&mut *this.writer).poll_write(cx, this.buf))?;
            {
                // Shorten the buffer according to the number of written bytes
                let (_, rest) = mem::take(&mut this.buf).split_at(n);
                this.buf = rest;
            }
            // If `poll_write` returns 0 bytes before the buffer was emptied means the Writer was closed before all the data could be written
            if n == 0 {
                return Poll::Ready(Err(TransportError::ConnectionDrop));
            }
        }

        Poll::Ready(Ok(()))
    }
}

impl<'a, R: AsyncRead + ?Sized + Unpin> Future for ReadExact<'a, R> {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        // Will shrink the slice of the buffer while filling it until the slice is empty
        while !this.buf.is_empty() {
            // Reads from the Reader into the slice if ready or returns Pending
            let n = ready!(Pin::new(&mut *this.reader).poll_read(cx, this.buf))?;
            // Shrinks the slice according to the number of bytes read.
            {
                let (_, rest) = mem::take(&mut this.buf).split_at_mut(n);
                this.buf = rest;
            }

            // If the slice isn't empty but the Reader return 0 it's because we reach the end of the Reader before filling the buffer
            if n == 0 {
                return Poll::Ready(Err(TransportError::UnexpectedEof));
            }
        }
        Poll::Ready(Ok(()))
    }
}
