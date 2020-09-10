#[allow(unused)]

pub mod transport {
    use ockam_router::router::{MessageHandler};
    use ockam_message::message::{Message, Address};
    use std::net::UdpSocket;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::collections::HashMap;

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
