use crate::connection::TcpConnection;
use crate::traits::{Connection, Listener};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub struct OckamTcpListener {
    listener: TcpListener,
}

impl OckamTcpListener {
    pub async fn new(
        listen_address: std::net::SocketAddr,
    ) -> Result<Arc<Mutex<dyn Listener + Send>>, String> {
        let listener = TcpListener::bind(listen_address).await;
        match listener {
            Ok(l) => Ok(Arc::new(Mutex::new(OckamTcpListener { listener: l }))),
            Err(e) => {
                println!("******{:?}", e);
                Err(format!("failed to bind to {:?}", listen_address))
            }
        }
    }
}

#[async_trait]
impl Listener for OckamTcpListener {
    async fn accept(&mut self) -> Result<Arc<Mutex<dyn Connection + Send>>, String> {
        let stream = self.listener.accept().await;
        if stream.is_err() {
            Err("accept failed".into())
        } else {
            let (stream, _) = stream.unwrap();
            Ok(TcpConnection::new_from_stream(stream).await)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::connection::TcpConnection;
    use crate::listener::OckamTcpListener;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::runtime::{Builder, Runtime};
    use tokio::task;

    async fn client_worker() {
        let connection =
            TcpConnection::new(SocketAddr::from_str("127.0.0.1:4052").unwrap()).clone();
        let mut connection = connection.lock().await;
        connection.connect().await.unwrap();
        println!("client connected");
    }

    async fn listen_worker() {
        {
            let listener = OckamTcpListener::new(SocketAddr::from_str("127.0.0.1:4052").unwrap())
                .await
                .unwrap();
            let mut listener = listener.lock().await;
            let _connection = listener.accept().await.unwrap();
            println!("...listener accepted connection");
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
                    return;
                });

                let j2 = task::spawn(async {
                    let f = client_worker();
                    f.await;
                    return;
                });
                let (r1, r2) = tokio::join!(j1, j2);
                if r1.is_err() {
                    assert!(false);
                }
                if r2.is_err() {
                    assert!(false);
                }
            })
        }
    }
}
