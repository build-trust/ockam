use crate::error::TransportError;
use crate::transport_traits::{Connection, Listener};
use crate::TcpConnection;
use async_trait::async_trait;
use tokio::net::TcpListener as TokioTcpListener;

pub struct TcpListener {
    listener: TokioTcpListener,
}

impl TcpListener {
    /// Creates a [`Listener`] trait object reference for TCP.
    ///
    /// # Examples
    /// ```
    /// use ockam_transport_tcp::listener::TcpListener;
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// use tokio::runtime::{Builder, Runtime};
    ///
    /// let runtime = Builder::new_current_thread().enable_io().build().unwrap();
    /// runtime.block_on( async {
    ///    let address = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    ///    let listener = TcpListener::new(address).await.unwrap();
    /// });
    /// ```
    pub async fn new(
        listen_address: std::net::SocketAddr,
    ) -> Result<Box<dyn Listener + Send>, TransportError> {
        let listener = TokioTcpListener::bind(listen_address).await;
        match listener {
            Ok(l) => Ok(Box::new(TcpListener { listener: l })),
            Err(_) => Err(TransportError::Bind),
        }
    }
}

#[async_trait]
impl Listener for TcpListener {
    /// Accepts an incoming connection request and returns a [`Connection`]
    /// trait object reference.
    ///
    /// # Examples
    /// ```ignore
    /// use ockam_transport_tcp::listener::TcpListener;
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    ///         
    /// let address = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    /// let mut  listener = TcpListener::new(address).await.unwrap();
    /// let connection = listener.accept().await.unwrap();
    /// ```
    async fn accept(&mut self) -> Result<Box<dyn Connection + Send>, TransportError> {
        let stream = self.listener.accept().await;
        if stream.is_err() {
            Err(TransportError::Accept)
        } else {
            let (stream, _) = stream.unwrap();
            Ok(TcpConnection::new_from_stream(stream).await?)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::connection::TcpConnection;
    use crate::listener::TcpListener;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::runtime::{Builder, Runtime};
    use tokio::task;

    async fn client_worker() {
        let mut connection = TcpConnection::new(SocketAddr::from_str("127.0.0.1:4052").unwrap());
        connection.connect().await.unwrap();
    }

    async fn listen_worker() {
        {
            let mut listener = TcpListener::new(SocketAddr::from_str("127.0.0.1:4052").unwrap())
                .await
                .unwrap();
            let _connection = listener.accept().await.unwrap();
        }
    }
    #[test]
    pub fn connect() {
        let runtime: [Runtime; 2] = [
            Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("ockam-tcp")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_io()
                .build()
                .unwrap(),
            Builder::new_current_thread().enable_io().build().unwrap(),
        ];

        for r in runtime.iter() {
            r.block_on(async {
                let j1 = task::spawn(async {
                    let f = listen_worker();
                    f.await;
                });

                let j2 = task::spawn(async {
                    let f = client_worker();
                    f.await;
                });
                let (r1, r2) = tokio::join!(j1, j2);
                if r1.is_err() {
                    panic!();
                }
                if r2.is_err() {
                    panic!();
                }
            })
        }
    }
}
