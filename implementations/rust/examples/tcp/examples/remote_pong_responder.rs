//! Spawn two workers that play some ping-pong over TCP

use ockam::Address;
use ockam_router::message::{Route, RouterAddress, ROUTER_ADDRESS_LOCAL};
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::{Player, PlayerMessage};

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
        ctx.start_worker(address.clone(), player).await.unwrap();
        ctx.send_message(address.clone(), PlayerMessage::Return)
            .await
            .unwrap();
    })
    .unwrap();
}
