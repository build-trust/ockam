#![allow(unused)]
use ockam_router::message::{Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_TCP};
use ockam_transport_tcp::connection::TcpConnection;
use ockam_transport_tcp::error::TransportError;
use ockam_transport_tcp::listener::TcpListener;
use ockam_transport_tcp::transport_traits::Connection;
use serde_bare;
use serde_bare::error::Error::Message;
use std::env;
use std::net::SocketAddr;
use std::process;
use std::str::FromStr;
use tokio::runtime::Builder;
use tokio::time::Duration;

pub async fn run_test(mut c: Box<dyn Connection>, r: Option<RouterAddress>, listener: bool) {
    let mut remote_addr = RouterAddress {
        address_type: ROUTER_ADDRESS_TCP,
        address: vec![],
    };
    if listener {
        let m = c.receive_message().await.unwrap();
        println!("{}", String::from_utf8(m.payload).unwrap());
        remote_addr = c.get_remote_address();
    } else {
        remote_addr = r.unwrap();
    }
    let local_addr = c.get_local_address();
    loop {
        let mut buf = String::new();
        if std::io::stdin().read_line(&mut buf).is_ok() {
            let m = RouterMessage {
                version: 1,
                onward_route: Route {
                    addrs: vec![remote_addr.clone()],
                },
                return_route: Route {
                    addrs: vec![local_addr.clone()],
                },
                payload: buf.as_bytes().to_vec(),
            };
            c.send_message(m).await.unwrap();
            let m = c.receive_message().await.unwrap();
            println!("{}", String::from_utf8(m.payload).unwrap());
        } else {
            return;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    println!("{:?}", args);

    let runtime = Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    println!("{:?}", args);

    runtime.block_on(async {
        if args[1] == "r" {
            let mut l = TcpListener::create(std::net::SocketAddr::from_str(&args[2]).unwrap())
                .await
                .unwrap();
            let c = l.accept().await.unwrap();
            println!(
                "responder connected, local address: {:?}",
                c.get_local_address()
            );
            run_test(c, None, true).await;
        } else {
            let mut c = TcpConnection::create(std::net::SocketAddr::from_str(&args[2]).unwrap());
            c.connect().await;
            println!(
                "initiator connected to remote address: {:?}",
                c.get_remote_address()
            );

            let remote_addr = serde_bare::to_vec::<SocketAddr>(
                &std::net::SocketAddr::from_str(&args[2]).unwrap(),
            )
            .unwrap();
            run_test(
                c,
                Some(RouterAddress {
                    address_type: ROUTER_ADDRESS_TCP,
                    address: remote_addr,
                }),
                false,
            )
            .await;
        }
    });
}
