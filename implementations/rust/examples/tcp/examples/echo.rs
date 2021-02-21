use ockam::Address;
use ockam_router::message::{Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_LOCAL};
use ockam_transport_tcp::TcpConnection;
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::{Player, PlayerMessage};

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let connect_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut connection = TcpConnection::create(connect_addr);
        connection.connect().await.unwrap();
        println!("connected to {:?}", connection.get_remote_address());

        let mut message = RouterMessage::new();
        message.onward_address(RouterAddress::Local(b"echoer".to_vec()));

        ctx.start_worker(address, player).await.unwrap();
        println!("initiator started");
        ctx.send_message(
            "initiator",
            PlayerMessage::Serve(Route {
                addrs: vec![RouterAddress {
                    address_type: ROUTER_ADDRESS_LOCAL,
                    address: "receiver".into(),
                }],
            }),
        )
        .await
        .unwrap();
        println!("serve sent");
    })
    .unwrap();
}
