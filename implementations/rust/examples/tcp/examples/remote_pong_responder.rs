#![allow(unused)]
//! Spawn two workers that play some ping-pong

use ockam::{Address, Context, Result, Worker};
use ockam_router::message::{Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_LOCAL};
use ockam_transport_tcp::{Connection, Listener};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::{Player, PlayerMessage};
use tokio::net::TcpListener;

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let listen_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut listener = ockam_transport_tcp::TcpListener::create(listen_addr)
            .await
            .unwrap();
        let connection = listener.accept().await.unwrap();
        println!(
            "Connected to {:?} on {:?}",
            connection.get_remote_address(),
            connection.get_local_address()
        );
        let player = Player {
            connection,
            return_route: Route {
                addrs: vec![RouterAddress {
                    address_type: ROUTER_ADDRESS_LOCAL,
                    address: "receiver".into(),
                }],
            },
            counter: 0,
        };
        let address: Address = "receiver".into();
        ctx.start_worker(address.clone(), player);
        ctx.send_message(address.clone(), PlayerMessage::Return)
            .await
            .unwrap();
    })
    .unwrap();
}
