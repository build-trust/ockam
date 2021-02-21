use ockam::Address;
use ockam_transport_tcp::TcpConnection;
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::echoer::{EchoMessage, Echoer};

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let connect_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut connection = TcpConnection::create(connect_addr);
        connection.connect().await.unwrap();

        let echoer = Echoer {
            connection,
            count: 0,
        };

        let address: Address = "echoer".into();
        ctx.start_worker(address, echoer).await.unwrap();

        ctx.send_message("echoer", EchoMessage::Send("Hello".into()))
            .await
            .unwrap();

        ctx.send_message("echoer", EchoMessage::Receive)
            .await
            .unwrap();
    })
    .unwrap();
}
