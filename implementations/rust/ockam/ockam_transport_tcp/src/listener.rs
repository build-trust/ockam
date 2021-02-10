use crate::connection::TcpConnection;
use crate::error::Error;
use crate::traits::{Connection, Listener};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::Mutex;

pub struct TcpListener {
    listener: TokioTcpListener,
}

impl TcpListener {
    /// Creates a [`Listener`] trait object reference for TCP.
    ///
    /// # Examples
    /// ```ignore
    /// use ockam_transport_tcp::listener::TcpListener;
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    ///
    /// let address = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    /// let listener = TcpListener::create(address);
    /// ```
    pub async fn create(
        listen_address: std::net::SocketAddr,
    ) -> Result<Arc<Mutex<dyn Listener + Send>>, Error> {
        let listener = TokioTcpListener::bind(listen_address).await;
        match listener {
            Ok(l) => Ok(Arc::new(Mutex::new(TcpListener { listener: l }))),
            Err(_) => Err(Error::Bind),
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
    /// let listener = TcpListener::create(address);
    /// let connection = listener.accept().await.unwrap();
    /// ```
    async fn accept(&mut self) -> Result<Arc<Mutex<dyn Connection + Send>>, String> {
        let stream = self.listener.accept().await;
        if stream.is_err() {
            Err(Error::Accept)
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
        let connection =
            TcpConnection::create(SocketAddr::from_str("127.0.0.1:4052").unwrap()).clone();
        let mut connection = connection.lock().await;
        connection.connect().await.unwrap();
    }

    async fn listen_worker() {
        {
            let listener = TcpListener::create(SocketAddr::from_str("127.0.0.1:4052").unwrap())
                .await
                .unwrap();
            let mut listener = listener.lock().await;
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
