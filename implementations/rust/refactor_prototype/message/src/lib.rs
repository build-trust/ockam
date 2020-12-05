#![allow(unused)]

// Definition and implementation of an Ockam message and message components.
// Each message component, and the message overall, implements the "Codec" trait
// allowing it to be encoded/decoded for transmission over a transport.

pub const MAX_MESSAGE_SIZE: usize = 16348;

pub mod message {
    use crate::message::Address::ChannelAddress;
    use crate::message::MessageType::Payload;
    use hex::*;
    use std::convert::{Into, TryFrom};
    use std::error::Error;
    use std::fmt::Formatter;
    pub use std::io::{ErrorKind, Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::ops::Add;
    use std::slice;
    use std::str::FromStr;

    const WIRE_PROTOCOL_VERSION: u8 = 1;

    /// If the message needs additional routing, return Ok(Some(msg))
    pub trait Receiver {
        fn recv(&mut self, m: Message) -> Result<Option<Message>, String>;
    }

    pub trait Sender {
        fn send(&mut self, m: Message) -> bool;
    }

    pub trait Codec {
        type Inner;

        fn encode(&self, v: &mut Vec<u8>) -> Result<(), String>;
        fn decode(s: &[u8]) -> Result<(Self::Inner, &[u8]), String>;
    }

    //    #[repr(C)]
    #[derive(Debug, Clone)]
    pub struct Message {
        pub onward_route: Route,
        pub return_route: Route,
        pub message_type: MessageType,
        pub message_body: Vec<u8>,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum MessageType {
        Ping = 0,
        Pong = 1,
        Payload = 2,
        KeyAgreementM1 = 3,
        KeyAgreementM2 = 4,
        KeyAgreementM3 = 5,
        NoSuchChannel = 9,
        None = 255,
    }

    impl Default for Message {
        fn default() -> Message {
            Message {
                onward_route: Route { addresses: vec![] },
                return_route: Route { addresses: vec![] },
                message_type: Payload,
                message_body: vec![0],
            }
        }
    }

    impl Codec for Message {
        type Inner = Message;
        fn encode(&self, u: &mut Vec<u8>) -> Result<(), String> {
            u.push(1);
            Route::encode(&self.onward_route.clone(), u);
            Route::encode(&self.return_route.clone(), u);
            u.push(self.message_type as u8);
            u.extend(&self.message_body[0..]);
            Ok(())
        }

        fn decode(u: &[u8]) -> Result<(Message, &[u8]), String> {
            let mut msg = Message::default();
            let mut w = &u[1..];
            match Route::decode(w) {
                Ok((r, u1)) => {
                    msg.onward_route = r;
                    w = u1;
                }
                Err(s) => {
                    return Err(s);
                }
            }
            match Route::decode(w) {
                Ok((r, u1)) => {
                    msg.return_route = r;
                    w = u1;
                }
                Err(s) => {
                    return Err(s);
                }
            }
            msg.message_type = MessageType::try_from(w[0])?;
            let mut w = &w[1..];
            msg.message_body = w.to_vec();
            Ok((msg, w))
        }
    }

    /* Addresses */
    #[derive(Debug, PartialEq)]
    //    #[repr(C)]
    #[derive(Clone)]
    pub struct RouterAddress {
        pub a_type: AddressType,
        pub length: u8,
        pub address: Address,
    }

    impl Clone for AddressType {
        fn clone(&self) -> Self {
            match self {
                AddressType::Tcp => AddressType::Tcp,
                AddressType::Udp => AddressType::Udp,
                AddressType::Channel => AddressType::Channel,
                AddressType::Worker => AddressType::Worker,
                AddressType::Undefined => AddressType::Undefined,
            }
        }
    }

    //    #[repr(C)]
    #[derive(Clone, Debug, PartialEq)]
    pub enum Address {
        TcpAddress(SocketAddr),
        UdpAddress(SocketAddr),
        ChannelAddress(Vec<u8>),
        WorkerAddress(Vec<u8>),
    }

    impl Address {
        pub fn as_string(&self) -> String {
            match self {
                Address::UdpAddress(socket) => socket.to_string(),
                Address::TcpAddress(socket) => socket.to_string(),
                Address::ChannelAddress(u) | Address::WorkerAddress(u) => hex::encode(u.as_slice()),
                _ => "error".to_string(),
            }
        }
        pub fn worker_address_from_string(s: &str) -> Result<Address, String> {
            match hex::decode(s) {
                Ok(h) => Ok(Address::WorkerAddress(h)),
                _ => Err("string must only contain hex digits".into()),
            }
        }
        pub fn channel_address_from_string(s: &str) -> Result<Address, String> {
            match hex::decode(s) {
                Ok(h) => Ok(Address::ChannelAddress(h)),
                _ => Err("string must only contain hex digits".into()),
            }
        }
        pub fn size_of(&self) -> u8 {
            match self {
                Address::WorkerAddress(a) => a.len() as u8,
                Address::UdpAddress(s) => 7,
                Address::TcpAddress(s) => 7,
                Address::ChannelAddress(a) => a.len() as u8,
                _ => 0,
            }
        }
    }

    pub enum HostAddressType {
        Ipv4 = 0,
        Ipv6 = 1,
    }

    #[derive(Copy)]
    pub enum AddressType {
        Undefined = 255,
        Tcp = 1,
        Udp = 2,
        Channel = 129,
        Worker = 0,
    }

    impl std::fmt::Debug for AddressType {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            let s: String;
            match self {
                AddressType::Tcp => {
                    s = "Tcp".to_string();
                }
                AddressType::Udp => {
                    s = "Udp".to_string();
                }
                AddressType::Channel => {
                    s = "Channel".to_string();
                }
                AddressType::Worker => {
                    s = "worker".to_string();
                }
                AddressType::Undefined => {
                    s = "Undefined".to_string();
                }
            }
            f.debug_struct("AddressType").field("Type", &s).finish();
            Ok(())
        }
    }

    impl PartialEq for AddressType {
        fn eq(&self, other: &Self) -> bool {
            let t: u8 = *self as u8;
            let o = *other as u8;
            o == t
        }
    }

    impl TryFrom<u8> for MessageType {
        type Error = String;
        fn try_from(data: u8) -> Result<Self, Self::Error> {
            match data {
                0 => Ok(MessageType::Ping),
                1 => Ok(MessageType::Pong),
                2 => Ok(MessageType::Payload),
                3 => Ok(MessageType::KeyAgreementM1),
                4 => Ok(MessageType::KeyAgreementM2),
                5 => Ok(MessageType::KeyAgreementM3),
                _ => Err("Unknown message type".to_string()),
            }
        }
    }

    impl TryFrom<u8> for HostAddressType {
        type Error = String;
        fn try_from(data: u8) -> Result<Self, Self::Error> {
            match data {
                0 => Ok(HostAddressType::Ipv4),
                1 => Ok(HostAddressType::Ipv6),
                _ => Err("Unknown host address type".to_string()),
            }
        }
    }

    impl TryFrom<u8> for AddressType {
        type Error = String;
        fn try_from(data: u8) -> Result<AddressType, Self::Error> {
            match data {
                255 => Ok(AddressType::Undefined),
                1 => Ok(AddressType::Tcp),
                2 => Ok(AddressType::Udp),
                129 => Ok(AddressType::Channel),
                0 => Ok(AddressType::Worker),
                _ => Err("Unknown address type".to_string()),
            }
        }
    }

    impl Codec for RouterAddress {
        type Inner = RouterAddress;
        fn encode(&self, v: &mut Vec<u8>) -> Result<(), String> {
            v.push(self.a_type as u8);
            v.push(self.length as u8);

            match self.a_type {
                AddressType::Worker => {
                    if let Address::WorkerAddress(mut wa) = self.address.clone() {
                        v.append(&mut wa);
                    }
                }
                AddressType::Udp => {
                    if let Address::UdpAddress(sock_addr) = self.address.clone() {
                        SocketAddr::encode(&sock_addr, v);
                    }
                }
                AddressType::Tcp => {
                    if let Address::TcpAddress(sock_addr) = self.address.clone() {
                        SocketAddr::encode(&sock_addr, v);
                    }
                }
                AddressType::Channel => {
                    if let Address::ChannelAddress(mut ca) = self.address.clone() {
                        v.append(&mut ca);
                    }
                }
                _ => {}
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(RouterAddress, &[u8]), String> {
            let mut a_type: AddressType;
            a_type = AddressType::Undefined;
            match AddressType::try_from(u[0]) {
                Ok(t) => {
                    a_type = t;
                }
                Err(s) => return Err(s),
            }
            match a_type {
                AddressType::Channel => {
                    let length = u[1] as usize;
                    let addr: Vec<u8> = u[2..(length + 2)].to_vec();
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Channel,
                            length: addr.len() as u8,
                            address: Address::ChannelAddress(addr),
                        },
                        &u[(length + 2)..],
                    ))
                }
                AddressType::Worker => {
                    let length = u[1] as usize;
                    let addr: Vec<u8> = u[2..(length + 2)].to_vec();
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Worker,
                            length: addr.len() as u8,
                            address: Address::WorkerAddress(addr),
                        },
                        &u[(length + 2)..],
                    ))
                }
                AddressType::Udp => {
                    let (sock, v) = SocketAddr::decode(&u[2..])?;
                    let address = Address::UdpAddress(sock);
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Udp,
                            length: u[1],
                            address: Address::UdpAddress(sock),
                        },
                        &u[u[1] as usize + 2..],
                    ))
                }
                AddressType::Tcp => {
                    let (sock, v) = SocketAddr::decode(&u[2..])?;
                    let address = Address::TcpAddress(sock);
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Tcp,
                            length: u[1],
                            address: Address::TcpAddress(sock),
                        },
                        &u[u[1] as usize + 2..],
                    ))
                }
                _ => Err("unimplemented address type".to_string()),
            }
        }
    }

    impl Codec for IpAddr {
        type Inner = IpAddr;
        fn encode(&self, v: &mut Vec<u8>) -> Result<(), String> {
            match self {
                std::net::IpAddr::V4(ip4) => {
                    v.push(HostAddressType::Ipv4 as u8);
                    v.extend_from_slice(ip4.octets().as_ref());
                }
                std::net::IpAddr::V6(ip6) => {
                    v.push(HostAddressType::Ipv6 as u8);
                    v.extend_from_slice(ip6.octets().as_ref());
                }
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(IpAddr, &[u8]), String> {
            match (HostAddressType::try_from(u[0])?, &u[1..]) {
                (HostAddressType::Ipv4, addr) => {
                    let ip4 = Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]);
                    let ip_addr = IpAddr::V4(ip4);
                    Ok((ip_addr, &u[5..]))
                }
                _ => Err("".to_string()),
            }
        }
    }

    impl Codec for SocketAddr {
        type Inner = SocketAddr;
        fn encode(&self, v: &mut Vec<u8>) -> Result<(), String> {
            match self {
                std::net::SocketAddr::V4(sock4) => {
                    v.push(HostAddressType::Ipv4 as u8);
                    v.extend_from_slice(sock4.ip().octets().as_ref());
                    let p = sock4.port();
                    v.extend_from_slice(&p.to_le_bytes());
                }
                std::net::SocketAddr::V6(sock6) => {
                    v.push(HostAddressType::Ipv6 as u8);
                    v.extend_from_slice(sock6.ip().octets().as_ref());
                    let p = sock6.port();
                    v.extend_from_slice(&p.to_le_bytes());
                }
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(SocketAddr, &[u8]), String> {
            match (HostAddressType::try_from(u[0])?, &u[1..]) {
                (HostAddressType::Ipv4, addr) => {
                    let ip4 = Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]);
                    let port = u16::from_le_bytes([addr[4], addr[5]]);
                    let sock = SocketAddr::new(IpAddr::V4(ip4), port);
                    Ok((sock, &addr[6..]))
                }
                _ => Err("".to_string()),
            }
        }
    }

    impl Route {
        pub fn print_route(&self) {
            for a in &self.addresses {
                match &a.address {
                    Address::UdpAddress(udp) => {
                        println!("Udp: {}", udp.to_string());
                    }
                    Address::TcpAddress(tcp) => {
                        println!("Tcp: {}", tcp.to_string());
                    }
                    Address::WorkerAddress(wa) => {
                        println!("worker: {}", hex::encode(wa));
                    }
                    Address::ChannelAddress(ca) => {
                        println!("Channel: {}", hex::encode(ca));
                    }
                    _ => {
                        println!("print_route not implemented for type");
                    }
                }
            }
        }
    }

    impl RouterAddress {
        pub fn size_of(&self) -> u8 {
            match &self.address {
                Address::WorkerAddress(a) => a.len() as u8,
                Address::UdpAddress(_unused) => 7,
                Address::TcpAddress(_unused) => 7,
                Address::ChannelAddress(a) => a.len() as u8,
                _ => 0,
            }
        }
        pub fn from_address(a: Address) -> Option<RouterAddress> {
            match &a {
                Address::UdpAddress(sock_addr) => Some(RouterAddress {
                    a_type: AddressType::Udp,
                    length: a.size_of(),
                    address: Address::UdpAddress(*sock_addr),
                }),
                Address::TcpAddress(sock_addr) => Some(RouterAddress {
                    a_type: AddressType::Tcp,
                    length: a.size_of(),
                    address: Address::TcpAddress(*sock_addr),
                }),
                Address::ChannelAddress(ca) => Some(RouterAddress {
                    a_type: AddressType::Channel,
                    length: ca.len() as u8,
                    address: Address::ChannelAddress(ca.clone()),
                }),
                Address::WorkerAddress(ca) => Some(RouterAddress {
                    a_type: AddressType::Worker,
                    length: ca.len() as u8,
                    address: Address::WorkerAddress(ca.clone()),
                }),
                _ => None,
            }
        }
        pub fn udp_router_address_from_str(s: &str) -> Result<RouterAddress, String> {
            match SocketAddr::from_str(s) {
                Ok(s) => Ok(RouterAddress {
                    a_type: AddressType::Udp,
                    length: 7,
                    address: Address::UdpAddress(s),
                }),
                Err(_unused) => Err("failed to parse router address".to_string()),
            }
        }
        pub fn tcp_router_address_from_str(s: &str) -> Result<RouterAddress, String> {
            match SocketAddr::from_str(s) {
                Ok(s) => Ok(RouterAddress {
                    a_type: AddressType::Tcp,
                    length: 7,
                    address: Address::TcpAddress(s),
                }),
                Err(_unused) => Err("failed to parse router address".to_string()),
            }
        }
        pub fn channel_router_address_from_str(a: &str) -> Result<RouterAddress, String> {
            match hex::decode(a) {
                Ok(h) => Ok(RouterAddress {
                    a_type: AddressType::Channel,
                    length: h.len() as u8,
                    address: Address::ChannelAddress(h),
                }),
                Err(_unused) => Err("string contains non-hex chars".to_string()),
            }
        }
        pub fn worker_router_address_from_str(a: &str) -> Result<RouterAddress, String> {
            match hex_vec_from_str(a) {
                Ok(h) => Ok(RouterAddress {
                    a_type: AddressType::Worker,
                    length: h.len() as u8,
                    address: Address::WorkerAddress(h),
                }),
                Err(_unused) => Err("invalid hex input".to_string()),
            }
        }
    }

    /* Routes */
    //    #[repr(C)]
    #[derive(Debug)]
    pub struct Route {
        pub addresses: Vec<RouterAddress>,
    }

    impl Clone for Route {
        fn clone(&self) -> Self {
            Route {
                addresses: self.addresses.clone(),
            }
        }

        fn clone_from(&mut self, source: &Self) {
            unimplemented!()
        }
    }

    impl Codec for Route {
        type Inner = Route;
        fn encode(&self, u: &mut Vec<u8>) -> Result<(), String> {
            if self.addresses.is_empty() {
                u.push(0 as u8)
            } else {
                u.push(self.addresses.len() as u8);
                for i in 0..self.addresses.len() {
                    RouterAddress::encode(&self.addresses[i].clone(), u);
                }
            }
            Ok(())
        }
        fn decode(encoded: &[u8]) -> Result<(Route, &[u8]), String> {
            let mut route = Route { addresses: vec![] };
            let mut next_address = &encoded[1..];
            if 0 < encoded[0] {
                for i in 0..encoded[0] as usize {
                    match RouterAddress::decode(next_address) {
                        Ok((a, x)) => {
                            route.addresses.push(a);
                            next_address = x;
                        }
                        Err(s) => {}
                    }
                }
            }
            Ok((route, next_address))
        }
    }

    // ToDo: Implement PartialEq, Eq, Copy, Clone

    // u16's are encoded as variable-length.
    // - If the value is < 0x80, it is encoded as-is, in one byte
    // - If the value is <= 0x80, the highest-order of the low-order byte is moved to the
    //   lowest-order bit in the high-order byte, and the high-order byte is shifted left by one to
    //   make room.

    pub fn varint_size(i: u16) -> usize {
        return if i < 0x80 { 1 } else { 2 };
    }

    impl Codec for u16 {
        type Inner = u16;
        fn encode(&self, u: &mut Vec<u8>) -> Result<(), String> {
            if self >= &0xC000 {
                return Err("Maximum value exceeded".to_string());
            }
            let mut bytes = self.to_le_bytes();

            if self < &0x80 {
                u.push(bytes[0])
            } else {
                bytes[1] <<= &0x01;
                if 0 != (bytes[0] & 0x80) {
                    bytes[1] |= 0x01;
                }
                bytes[0] |= &0x80;
                u.push(bytes[0]);
                u.push(bytes[1])
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(Self::Inner, &[u8]), String> {
            let mut bytes = [0, 0];
            let mut i = 1;

            bytes[0] = u[0] & 0x7f;
            if (u[0] & 0x80) == 0x80 as u8 {
                bytes[0] += (u[1] & 0x01) << 7;
                bytes[1] = u[1] >> 1;
                i = 2;
            }
            let ul2 = ((bytes[1] as u16) << 8) + bytes[0] as u16;

            Ok((ul2, &u[i..]))
        }
    }

    #[derive(Debug)]
    //    #[repr(C)]
    pub struct WireProtocolVersion {
        pub v: u16,
    }

    impl Default for WireProtocolVersion {
        fn default() -> WireProtocolVersion {
            WireProtocolVersion { v: 1 }
        }
    }

    // std::io::Read & std::io::Write trait implementation
    impl std::io::Read for Message {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            if buf.len() < self.message_body.len() {
                return Err(std::io::Error::new(ErrorKind::Other, "buffer too small"));
            }
            buf[..self.message_body.len()].clone_from_slice(&self.message_body[..]);
            Ok(self.message_body.len())
        }
    }
    impl std::io::Write for Message {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            if !self.message_body.is_empty() {
                return Err(std::io::Error::new(ErrorKind::Other, "no message body"));
            }
            for b in buf {
                self.message_body.push(*b);
            }
            Ok(self.message_body.len())
        }
        fn flush(&mut self) -> Result<(), std::io::Error> {
            Ok(())
        }
    }

    pub fn hex_vec_from_str(s: &str) -> Result<Vec<u8>, String> {
        let mut hex: Vec<u8> = vec![];
        if s.len() % 2 != 0 {
            return Err("odd number of input chars".into());
        }
        for i in 0..s.len() {
            if 0 == i % 2 {
                let s2: &str = &s[i..(i + 2)];
                match u8::from_str_radix(s2, 16) {
                    Ok(val) => {
                        hex.push(val);
                    }
                    _ => {
                        return Err("non-hex characters found in string".into());
                    }
                }
            }
        }
        Ok(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::*;
    use hex::encode;
    use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr};
    use std::str::FromStr;

    #[test]
    fn test_router_address_from_string() {
        match RouterAddress::udp_router_address_from_str("127.0.0.1:8080") {
            Ok(ra) => {
                assert_eq!(ra.length, 7);
                assert_eq!(ra.a_type, AddressType::Udp);
                match ra.address {
                    Address::UdpAddress(sa) => {
                        assert_eq!(sa, SocketAddr::from_str("127.0.0.1:8080").unwrap());
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
            _ => {
                assert!(false);
            }
        }
        match RouterAddress::worker_router_address_from_str("01242020") {
            Ok(ra) => {
                assert_eq!(ra.length, 4);
                assert_eq!(ra.a_type, AddressType::Worker);
                match ra.address {
                    Address::WorkerAddress(wa) => {
                        assert_eq!(hex::encode(&wa), "01242020");
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
            _ => {
                assert!(false);
            }
        }
        match RouterAddress::worker_router_address_from_str("01242020070707") {
            Ok(ra) => {
                assert_eq!(ra.length, 7);
                assert_eq!(ra.a_type, AddressType::Worker);
                match ra.address {
                    Address::WorkerAddress(wa) => {
                        assert_eq!(wa, vec![1, 36, 32, 32, 7, 7, 7]);
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn ip4_address_codec() {
        let mut v: Vec<u8> = vec![];
        let mut ip4a: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        IpAddr::encode(&ip4a, &mut v);
        assert_eq!(v, vec![0, 127, 0, 0, 1]);
        let mut v: Vec<u8> = vec![0, 127, 0, 0, 1];
        match IpAddr::decode(&v) {
            Ok((ip4a, w)) => {
                assert_eq!(ip4a, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
            }
            Err(s) => {
                println!("{}", s);
            }
        }
    }

    #[test]
    fn address_codec() {
        // Socket address
        let sa = SocketAddr::from_str("127.0.0.1:32896").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address.clone(),
        };
        assert_eq!(udp_address.as_string(), "127.0.0.1:32896");
        router_address.length = router_address.size_of();
        let mut v: Vec<u8> = vec![];
        RouterAddress::encode(&router_address, &mut v);
        assert_eq!(v, vec![2, 7, 0, 127, 0, 0, 1, 0x80, 0x80]);

        let mut v = vec![2, 7, 0, 127, 0, 0, 1, 0x80, 0x80];
        match RouterAddress::decode(&mut v) {
            Ok((ra, w)) => {
                assert_eq!(ra.a_type, AddressType::Udp);
                assert_eq!(ra.length, 7);
                match ra.address {
                    Address::UdpAddress(sock_addr) => {
                        assert!(sock_addr.is_ipv4());
                        match sock_addr {
                            SocketAddr::V4(sock4) => {
                                assert_eq!(sock4.to_string(), "127.0.0.1:32896");
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(s) => {
                println!("{}", s);
            }
        }

        // Channel address
        let mut router_channel_address =
            RouterAddress::channel_router_address_from_str("00010203").unwrap();
        let mut v: Vec<u8> = vec![];
        RouterAddress::encode(&router_channel_address, &mut v);
        assert_eq!(v, vec![129, 4, 0, 1, 2, 3]);

        let mut v = vec![129, 4, 3, 2, 1, 0];
        match RouterAddress::decode(&mut v) {
            Ok((ra, _unused)) => {
                assert_eq!(ra.a_type, AddressType::Channel);
                assert_eq!(ra.length, 4);
                match ra.address {
                    Address::ChannelAddress(c) => {
                        assert_eq!(c, vec![3 as u8, 2 as u8, 1 as u8, 0 as u8]);
                    }
                    _ => {}
                }
            }
            Err(s) => {
                println!("{}", s);
            }
        }
    }

    #[test]
    fn route_codec() {
        let mut route = Route { addresses: vec![] };
        let sa = SocketAddr::from_str("127.0.0.1:32896").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let sa = SocketAddr::from_str("10.0.1.10:32912").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(vec![0, 1, 2, 3]);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let mut v: Vec<u8> = vec![];
        Route::encode(&route, &mut v);
        assert_eq!(
            v,
            vec![
                3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90, 0x80, 129, 4, 0,
                1, 2, 3
            ]
        );
        match Route::decode(&v) {
            Ok((r, u)) => {
                assert_eq!(r.addresses.len(), 3);

                match r.addresses[0].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, r.addresses[0].length);
                        match r.addresses[0].address {
                            Address::UdpAddress(sock_addr) => {
                                assert!(sock_addr.is_ipv4());
                                match sock_addr {
                                    SocketAddr::V4(sock4) => {
                                        assert_eq!(sock4.to_string(), "127.0.0.1:32896");
                                    }
                                    _ => {
                                        assert!(false);
                                    }
                                }
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => assert!(false),
                }
                match r.addresses[1].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, r.addresses[1].length);
                        match r.addresses[1].address {
                            Address::UdpAddress(sock_addr) => {
                                assert!(sock_addr.is_ipv4());
                                match sock_addr {
                                    SocketAddr::V4(sock4) => {
                                        assert_eq!(sock4.to_string(), "10.0.1.10:32912");
                                    }
                                    _ => {
                                        assert!(false);
                                    }
                                }
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => assert!(false),
                }
                match r.addresses[2].a_type {
                    AddressType::Channel => {
                        assert_eq!(r.addresses[2].length, 4);
                        match &r.addresses[2].address {
                            Address::ChannelAddress(a) => {
                                assert_eq!(a, &vec![0 as u8, 1 as u8, 2 as u8, 3 as u8]);
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => {
                        assert!(false);
                    }
                }

                assert_eq!(v.len(), 25);
            }
            Err(s) => {
                panic!();
            }
        }
    }

    #[test]
    fn u16_codec() {
        let mut u: Vec<u8> = vec![];
        let mut n: u16 = 0x7f;
        u16::encode(&n, &mut u);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0], 0x7f);
        match u16::decode(&u) {
            Ok((m, v)) => {
                assert_eq!(u[0], 0x7f);
                assert_eq!(v.len(), 0);
            }
            Err(s) => panic!(),
        }

        let mut u: Vec<u8> = vec![0x7f, 1, 2, 3];
        match u16::decode(&u) {
            Ok((m, v)) => {
                assert_eq!(v[0], 1);
                assert_eq!(v.len(), 3);
            }
            Err(s) => panic!(),
        }

        let mut too_big: u16 = 0xC000;
        let mut u: Vec<u8> = vec![];
        match u16::encode(&too_big, &mut u) {
            Ok(()) => panic!(),
            Err(s) => {}
        }

        let mut n = 0x80;
        let mut u: Vec<u8> = vec![];
        u16::encode(&n, &mut u);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0], 0x80);
        assert_eq!(u[1], 0x01);
        match u16::decode(&u[0..]) {
            Ok((m, v)) => {
                assert_eq!(m, 0x80);
                assert_eq!(v.len(), 0);
            }
            Err(e) => panic!(),
        }

        let mut n = 0x1300;
        let mut u: Vec<u8> = vec![];
        u16::encode(&n, &mut u);
        assert_eq!(u.len(), 2);
        assert_eq!(u[1], 0x13 << 1);
        assert_eq!(u[0], 0x80);
        match u16::decode(&u[0..]) {
            Ok((m, v)) => {
                assert_eq!(m, 0x1300);
                assert_eq!(v.len(), 0);
            }
            Err(e) => panic!(),
        }

        let mut n = 0x1381;
        let mut u: Vec<u8> = vec![];
        u16::encode(&n, &mut u);
        assert_eq!(u.len(), 2);
        assert_eq!(u[1], (0x13 << 1) | 1);
        assert_eq!(u[0], 0x81);
        match u16::decode(&u[0..]) {
            Ok((m, v)) => {
                assert_eq!(m, 0x1381);
                assert_eq!(v.len(), 0);
            }
            Err(e) => panic!(),
        }
    }

    #[test]
    fn message_codec() {
        let mut onward_route = Route { addresses: vec![] };
        let mut route = Route { addresses: vec![] };
        let sa = SocketAddr::from_str("127.0.0.1:32896").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let sa = SocketAddr::from_str("10.0.1.10:32912").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(vec![0, 1, 2, 3]);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let mut return_route = Route { addresses: vec![] };
        let sa = SocketAddr::from_str("127.0.0.1:32896").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let sa = SocketAddr::from_str("10.0.1.10:32912").unwrap();
        let mut udp_address = Address::UdpAddress(sa);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(vec![0, 1, 2, 3]);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let mut message_body = vec![1, 1, 1, 1];
        let mut msg = Message {
            onward_route,
            return_route,
            message_type: MessageType::Payload,
            message_body,
        };
        let mut u: Vec<u8> = vec![];
        Message::encode(&msg, &mut u);
        assert_eq!(
            u,
            vec![
                1, 3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90, 0x80, 129, 4,
                0, 1, 2, 3, 3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90,
                0x80, 129, 4, 0, 1, 2, 3, 2, 1, 1, 1, 1,
            ]
        );

        match Message::decode(&u) {
            Ok((m, v)) => {
                assert_eq!(m.onward_route.addresses.len(), 3);

                match m.onward_route.addresses[0].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, m.onward_route.addresses[0].length);
                        match m.onward_route.addresses[0].address {
                            Address::UdpAddress(sock_addr) => {
                                assert!(sock_addr.is_ipv4());
                                match sock_addr {
                                    SocketAddr::V4(sock4) => {
                                        assert_eq!(sock4.to_string(), "127.0.0.1:32896");
                                    }
                                    _ => {
                                        assert!(false);
                                    }
                                }
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => assert!(false),
                }
                match m.onward_route.addresses[1].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, m.onward_route.addresses[1].length);
                        match m.onward_route.addresses[1].address {
                            Address::UdpAddress(sock_addr) => {
                                assert!(sock_addr.is_ipv4());
                                match sock_addr {
                                    SocketAddr::V4(sock4) => {
                                        assert_eq!(sock4.to_string(), "10.0.1.10:32912");
                                    }
                                    _ => {
                                        assert!(false);
                                    }
                                }
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => assert!(false),
                }
                match m.onward_route.addresses[2].a_type {
                    AddressType::Channel => {
                        assert_eq!(m.onward_route.addresses[2].length, 4);
                        match &m.onward_route.addresses[2].address {
                            Address::ChannelAddress(a) => {
                                assert_eq!(a, &vec![0 as u8, 1 as u8, 2 as u8, 3 as u8]);
                            }
                            _ => {
                                assert!(false);
                            }
                        }
                    }
                    _ => {
                        assert!(false);
                    }
                }
                assert_eq!(m.message_type as u8, MessageType::Payload as u8);
                assert_eq!(&m.message_body[0..4], [1, 1, 1, 1]);
            }
            _ => {}
        }
    }
}
