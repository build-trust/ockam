use std::borrow::BorrowMut;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use tcp::connection::TcpConnection;
use tcp::listener::OckamTcpListener;
use tokio::runtime::{Builder, Runtime};
use tokio::task;

async fn client_worker() {
    let c1 = TcpConnection::new(SocketAddr::from_str("127.0.0.1:4052").unwrap()).clone();
    let mut c2 = c1.deref();
    let mut connection = c2.borrow_mut().lock().await;
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
        let mut listener = listener.deref();
        let mut listener = listener.borrow_mut().lock().await;
        let connection = listener.accept().await.unwrap();
        println!("...accepted connection");
        let mut connection = connection.deref();
        let mut connection = connection.borrow_mut().lock().await;
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

#[test]
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

            let _j2 = task::spawn(async {
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
