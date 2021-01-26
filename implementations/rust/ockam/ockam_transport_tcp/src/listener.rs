use crate::connection::TcpConnection;
use crate::traits::{Connection, Listener};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub struct OckamTcpListener {
    listener: Arc<Mutex<TcpListener>>,
}

impl OckamTcpListener {
    pub async fn new(
        listen_address: std::net::SocketAddr,
    ) -> Result<Arc<Mutex<dyn Listener + Send>>, String> {
        let listener = TcpListener::bind(listen_address).await;
        match listener {
            Ok(l) => Ok(Arc::new(Mutex::new(OckamTcpListener {
                listener: Arc::new(Mutex::new(l)),
            }))),
            Err(_) => Err(format!("failed to bind to {:?}", listen_address)),
        }
    }
}

#[async_trait]
impl Listener for OckamTcpListener {
    async fn accept(&mut self) -> Result<Arc<Mutex<dyn Connection + Send>>, String> {
        let listener = self.listener.lock().await;
        let stream = listener.accept().await;
        if stream.is_err() {
            Err("accept failed".into())
        } else {
            let (stream, _) = stream.unwrap();
            Ok(TcpConnection::new_from_stream(stream).await)
        }
    }

    fn stop(&mut self) {
        unimplemented!()
    }
}
