//! Spawn two workers that play some ping-pong

use ockam::{Address, Context, Result, Worker};
use ockam_router::message::{Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_LOCAL};
use ockam_transport_tcp::{Connection, Listener, TcpConnection};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::{Player, PlayerMessage};
use tokio::net::TcpListener;

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let connect_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut connection = TcpConnection::create(connect_addr);
        connection.connect().await.unwrap();
        println!("connected to {:?}", connection.get_remote_address());
        let player = Player {
            connection,
            return_route: Route {
                addrs: vec![RouterAddress {
                    address_type: ROUTER_ADDRESS_LOCAL,
                    address: "server".into(),
                }],
            },
            counter: 0,
        };
        let address: Address = "initiator".into();
        ctx.start_worker(address, player);
        println!("initiator started");
        ctx.send_message(
            "initiator",
            PlayerMessage::Serve(Route {
                addrs: vec![RouterAddress {
                    address_type: ROUTER_ADDRESS_LOCAL,
                    address: "receiver".into(),
                }],
            }),
        );
        println!("serve sent");
    })
    .unwrap();
}
