use ockam::Address;
use std::net::SocketAddr;
use std::str::FromStr;
use tcp_examples::printer::{Printer, PrinterMessage};

fn main() {
    let (ctx, mut exe) = ockam::start_node();

    exe.execute(async move {
        let listen_addr = SocketAddr::from_str("127.0.0.1:4051").unwrap();
        let mut listener = ockam_transport_tcp::TcpListener::create(listen_addr)
            .await
            .unwrap();
        let connection = listener.accept().await.unwrap();
        let printer = Printer {
            connection,
            count: 0,
        };

        let address: Address = "printer".into();
        ctx.start_worker(address, printer).await.unwrap();

        ctx.send_message("printer", PrinterMessage::Receive)
            .await
            .unwrap();

        ctx.send_message("printer", PrinterMessage::Send("Hello".into()))
            .await
            .unwrap();
    })
    .unwrap();
}
