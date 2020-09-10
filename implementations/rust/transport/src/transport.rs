#[allow(unused)]

pub mod transport {
    use ockam_message::message::{Address, Message};
    use ockam_router::router::MessageHandler;
    use std::collections::HashMap;
    use std::net::UdpSocket;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    pub struct UdpAddressHandler {
        pub socket: UdpSocket,
    }

    impl MessageHandler for UdpAddressHandler {
        fn message_handler(
            &self,
            mut m: Box<Message>,
            address: Address,
        ) -> Result<(), std::io::Error> {
            println!("In UdpAddressHandler!");
            Ok(())
        }
    }
}
