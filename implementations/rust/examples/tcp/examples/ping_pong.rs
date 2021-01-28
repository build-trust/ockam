use std::borrow::BorrowMut;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use tokio::runtime::{Builder, Runtime};
use tokio::task;
use ockam_transport_tcp::connection::TcpConnection;
use ockam_transport_tcp::listener::OckamTcpListener;

async fn client_worker() {
    let connection = TcpConnection::new(SocketAddr::from_str("127.0.0.1:4052").unwrap()).clone();
    let mut connection = connection.lock().await;
    match connection.connect().await {
        Err(_) => {
            assert!(false);
            return;
        }
        _ => {}
    }
    println!("client connected");
    let mut i = 0;
    loop {
        let mut buff: [u8; 32] = [0; 32];
        match connection.send(b"ping").await {
            Ok(_) => {}
            Err(s) => {
                println!("{}", s);
                assert!(false);
            }
        }
        match connection.receive(&mut buff).await {
            Ok(_) => {
                println!("{}", String::from_utf8(buff.to_vec()).unwrap());
            }
            Err(s) => {
                println!("{}", s);
                assert!(false);
            }
        }
        i += 1;
        if i == 5 {
            break;
        }
    }
    return;
}

async fn listen_worker() {
    {
        let listener = OckamTcpListener::new(SocketAddr::from_str("127.0.0.1:4052").unwrap())
            .await
            .unwrap();
        let mut listener = listener.lock().await;
        let connection = listener.accept().await.unwrap();
        println!("...accepted connection");
        let mut connection = connection.lock().await;
        let mut i = 0;
        loop {
            let mut buff: [u8; 32] = [0; 32];
            match connection.receive(&mut buff).await {
                Ok(_) => {
                    println!("{}", String::from_utf8(buff.to_vec()).unwrap());
                }
                Err(s) => {
                    println!("{}", s);
                    assert!(false);
                }
            }
            match connection.send(b"pong").await {
                Ok(_) => {}
                Err(s) => {
                    println!("{}", s);
                    assert!(false);
                }
            }
            i += 1;
            if i == 5 {
                break;
            }
        }
    }
}

pub fn main() {
    let runtime: [Runtime; 2] = [
        Builder::new_multi_thread().enable_io().build().unwrap(),
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
            tokio::join!(j1, j2);
        })
    }
}
