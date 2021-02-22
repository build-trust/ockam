use ockam::Address;
use ockam_transport_tcp::TcpConnection;
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::printer::{Printer, PrinterMessage};

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let connect_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut connection = TcpConnection::create(connect_addr);
        connection.connect().await.unwrap();

        let printer = Printer {
            connection,
            count: 0,
        };

        let address: Address = "printer".into();
        ctx.start_worker(address, printer).await.unwrap();

        ctx.send_message("printer", PrinterMessage::Send("Hello".into()))
            .await
            .unwrap();

        ctx.send_message("printer", PrinterMessage::Receive)
            .await
            .unwrap();
    })
    .unwrap();
}
