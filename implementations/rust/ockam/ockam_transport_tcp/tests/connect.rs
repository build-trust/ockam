use std::borrow::BorrowMut;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tcp::connection::TcpConnection;
use tcp::listener::OckamTcpListener;
use tcp::traits::Listener;
use tokio::net::TcpStream;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{Mutex, MutexGuard};
use tokio::task;

async fn client_worker() {
    let c1 = TcpConnection::new(SocketAddr::from_str("127.0.0.1:4052").unwrap()).clone();
    let mut c2 = c1.deref();
    let mut c3 = c2.borrow_mut().lock().await;
    let _connection = c3.connect().await.unwrap();
    println!("client connected");
}

async fn listen_worker() {
    {
        let listener = OckamTcpListener::new(SocketAddr::from_str("127.0.0.1:4052").unwrap())
            .await
            .unwrap();
        let mut listener = listener.deref();
        let mut listener = listener.borrow_mut().lock().await;
        let _connection = listener.accept().await.unwrap();
        println!("...listener accepted connection");
    }
}

#[test]
pub fn main() {
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
            })
            .await
            .unwrap();
            tokio::join!(j1);
        })
    }
}
