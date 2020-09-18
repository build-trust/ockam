#![allow(unused)]

// Definition and implementation of an Ockam message and message components.
// Each message component, and the message overall, implements the "Codec" trait
// allowing it to be encoded/decoded for transmission over a transport.

pub mod message {
    use std::convert::{Into, TryFrom};
    use std::error::Error;
    use std::fmt::Formatter;
    pub use std::io::{ErrorKind, Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::ops::Add;
    use std::slice;

    const WIRE_PROTOCOL_VERSION: u8 = 1;

    pub trait Codec {
        type Inner;

        fn encode(t: Self::Inner, v: &mut Vec<u8>) -> Result<(), String>;
        fn decode(s: &[u8]) -> Result<(Self::Inner, &[u8]), String>;
        fn decode_boxed(s: &[u8]) -> Result<(Box<Self::Inner>, &[u8]), String> {
            Err("not implemented".to_string())
        }
    }

    // #[derive(Debug)]
    #[repr(C)]
    pub struct Message {
        pub onward_route: Route,
        pub return_route: Route,
        pub message_body: Vec<u8>,
    }

    impl Default for Message {
        fn default() -> Message {
            Message {
                onward_route: Route { addresses: vec![] },
                return_route: Route { addresses: vec![] },
                message_body: vec![0],
            }
        }
    }

    impl Codec for Message {
        type Inner = Box<Message>;
        fn encode(msg: Box<Message>, u: &mut Vec<u8>) -> Result<(), String> {
            Route::encode(msg.onward_route, u);
            Route::encode(msg.return_route, u);
            u.extend(&msg.message_body[0..]);
            Ok(())
        }

        fn decode(u: &[u8]) -> Result<(Box<Message>, &[u8]), String> {
            let mut msg = Box::new(Message::default());
            let mut w = u;
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

            msg.message_body.append(&mut (w.to_vec()));
            Ok((msg, w))
        }
    }

    /* Addresses */
    #[repr(C)]
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct LocalAddress {
        pub address: u32,
    }

    #[derive(Clone, Copy)]
    pub struct RouterAddress {
        pub(crate) a_type: AddressType,
        pub(crate) length: u8,
        pub(crate) address: Address,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Address {
        // First two fields are type and length
        LocalAddress(LocalAddress),
        TcpAddress(IpAddr, u16),
        UdpAddress(IpAddr, u16),
        ChannelAddress(u32),
    }

    pub enum HostAddressType {
        Ipv4 = 0,
        Ipv6 = 1,
    }

    pub enum AddressType {
        Undefined = 255,
        Local = 0,
        Tcp = 1,
        Udp = 2,
        Channel = 129,
    }

    impl Clone for AddressType {
        fn clone(&self) -> Self {
            return match self {
                AddressType::Local => AddressType::Local,
                AddressType::Tcp => AddressType::Tcp,
                AddressType::Udp => AddressType::Udp,
                AddressType::Channel => AddressType::Channel,
                AddressType::Undefined => AddressType::Undefined,
            };
        }
    }

    impl std::fmt::Debug for AddressType {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            let s: String;
            match self {
                AddressType::Local => {
                    s = "Local".to_string();
                }
                AddressType::Tcp => {
                    s = "Tcp".to_string();
                }
                AddressType::Udp => {
                    s = "Udp".to_string();
                }
                AddressType::Channel => {
                    s = "Channel".to_string();
                }
                AddressType::Undefined => {
                    s = "Undefined".to_string();
                }
            }
            f.debug_struct("AddressType").field("Type", &s).finish();
            Ok(())
        }
    }

    impl Copy for AddressType {}

    impl PartialEq for AddressType {
        fn eq(&self, other: &Self) -> bool {
            let t: u8 = *self as u8;
            let o = *other as u8;
            return t == o;
        }

        fn ne(&self, other: &Self) -> bool {
            let t = *self as u8;
            let o = *other as u8;
            return t != o;
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
        fn try_from(data: u8) -> Result<Self, Self::Error> {
            match data {
                255 => Ok(AddressType::Undefined),
                0 => Ok(AddressType::Local),
                1 => Ok(AddressType::Tcp),
                2 => Ok(AddressType::Udp),
                129 => Ok(AddressType::Channel),
                _ => Err("Unknown address type".to_string()),
            }
        }
    }

    impl Codec for RouterAddress {
        type Inner = RouterAddress;
        fn encode(a: RouterAddress, v: &mut Vec<u8>) -> Result<(), String> {
            v.push(a.a_type as u8);
            v.push(a.length as u8);
            match a.a_type {
                AddressType::Local => match a.address {
                    Address::LocalAddress(la) => {
                        LocalAddress::encode(la, v);
                    }
                    _ => {}
                },
                AddressType::Udp => match a.address {
                    Address::UdpAddress(ipa, port) => {
                        IpAddr::encode(ipa, v);
                        v.append(&mut port.to_le_bytes().to_vec());
                    }
                    _ => {}
                },
                AddressType::Channel => match a.address {
                    Address::ChannelAddress(a) => {
                        v.append(&mut a.to_le_bytes().to_vec());
                    }
                    _ => {}
                },
                _ => {}
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(RouterAddress, &[u8]), String> {
            let mut a_type = AddressType::Undefined;
            match AddressType::try_from(u[0]) {
                Ok(t) => {
                    a_type = t;
                }
                Err(s) => return Err(s),
            }
            match a_type {
                AddressType::Channel => {
                    let address =
                        Address::ChannelAddress(u32::from_le_bytes([u[2], u[3], u[4], u[5]]));
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Channel,
                            length: u[1],
                            address,
                        },
                        &u[6..],
                    ))
                }
                AddressType::Udp => {
                    let (ipa, v) = IpAddr::decode(&u[2..])?;
                    let port = u16::from_le_bytes([v[0], v[1]]);
                    let address = Address::UdpAddress(ipa, port);
                    Ok((
                        RouterAddress {
                            a_type: AddressType::Udp,
                            length: u[1],
                            address,
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
        fn encode(ip: IpAddr, v: &mut Vec<u8>) -> Result<(), String> {
            match ip {
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

    impl Codec for LocalAddress {
        type Inner = LocalAddress;
        fn encode(la: LocalAddress, u: &mut Vec<u8>) -> Result<(), String> {
            for le_byte in la.address.to_le_bytes().iter() {
                u.push(*le_byte);
            }
            Ok(())
        }
        fn decode(u: &[u8]) -> Result<(LocalAddress, &[u8]), String> {
            Ok((
                LocalAddress {
                    address: u32::from_le_bytes([u[0], u[1], u[2], u[3]]),
                },
                &u[4..],
            ))
        }
    }

    impl RouterAddress {
        pub fn size_of(&self) -> u8 {
            match self.address {
                Address::LocalAddress(a) => 4,
                Address::UdpAddress(a, p) => 7,
                Address::ChannelAddress(a) => 4,
                _ => 0,
            }
        }
    }

    /* Routes */
    #[repr(C)]
    pub struct Route {
        pub addresses: Vec<RouterAddress>,
    }

    impl Clone for Route {
        fn clone(&self) -> Self {
            return Route {
                addresses: self.addresses.clone(),
            };
        }

        fn clone_from(&mut self, source: &Self) {
            unimplemented!()
        }
    }

    impl Codec for Route {
        type Inner = Route;
        fn encode(route: Route, u: &mut Vec<u8>) -> Result<(), String> {
            if route.addresses.is_empty() {
                u.push(0 as u8)
            } else {
                u.push(route.addresses.len() as u8);
                for i in 0..route.addresses.len() {
                    RouterAddress::encode(route.addresses[i], u);
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
    impl Codec for u16 {
        type Inner = u16;
        fn encode(ul2: u16, u: &mut Vec<u8>) -> Result<(), String> {
            if ul2 >= 0xC000 {
                return Err("Maximum value exceeded".to_string());
            }
            let mut bytes = ul2.to_le_bytes();

            if ul2 < 0x80 {
                u.push(bytes[0])
            } else {
                bytes[1] <<= 0x01;
                if 0 != (bytes[0] & 0x80) {
                    bytes[1] |= 0x01;
                }
                bytes[0] |= 0x80;
                u.push(bytes[0]);
                u.push(bytes[1])
            }
            Ok(())
        }

        fn decode(u: &[u8]) -> Result<(u16, &[u8]), String> {
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
    #[repr(C)]
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
            for i in 0..self.message_body.len() {
                buf[i] = self.message_body[i];
            }
            Ok(self.message_body.len())
        }
    }
    impl std::io::Write for Message {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            if !self.message_body.is_empty() {
                return Err(std::io::Error::new(ErrorKind::Other, "no message body"));
            }
            for i in 0..buf.len() {
                self.message_body.push(buf[i]);
            }
            Ok(self.message_body.len())
        }
        fn flush(&mut self) -> Result<(), std::io::Error> {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::*;
    use std::net::{AddrParseError, IpAddr, Ipv4Addr};

    #[test]
    fn local_address_codec() {
        let mut local_in = LocalAddress {
            address: 0x00010203,
        };
        let mut v: Vec<u8> = vec![];
        LocalAddress::encode(local_in, &mut v);
        assert_eq!(v, [3, 2, 1, 0]);
        match LocalAddress::decode(&v) {
            Ok((local_out, w)) => assert_eq!(
                local_out,
                LocalAddress {
                    address: 0x00010203
                }
            ),
            Err(s) => {
                println!("{:?}", s);
            }
        }
    }

    #[test]
    fn ip4_address_codec() {
        let mut v: Vec<u8> = vec![];
        let mut ip4a: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        IpAddr::encode(ip4a, &mut v);
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
        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        let mut v: Vec<u8> = vec![];
        RouterAddress::encode(router_address, &mut v);
        assert_eq!(v, vec![2, 7, 0, 127, 0, 0, 1, 0x80, 0x80]);
        let mut v = vec![2, 7, 0, 127, 0, 0, 1, 0x80, 0x80];
        match RouterAddress::decode(&mut v) {
            Ok((ra, w)) => {
                assert_eq!(ra.a_type, AddressType::Udp);
                assert_eq!(ra.length, 7);
                match ra.address {
                    Address::UdpAddress(ip, p) => {
                        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
                        assert_eq!(p, 0x8080);
                    }
                    _ => {}
                }
            }
            Err(s) => {
                println!("{}", s);
            }
        }
        let mut channel_address = Address::ChannelAddress(0x00010203);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        let mut v: Vec<u8> = vec![];
        RouterAddress::encode(router_address, &mut v);
        assert_eq!(v, vec![129, 4, 3, 2, 1, 0]);
        let mut v = vec![129, 4, 3, 2, 1, 0];
        match RouterAddress::decode(&mut v) {
            Ok((ra, _0)) => {
                assert_eq!(ra.a_type, AddressType::Channel);
                assert_eq!(ra.length, 4);
                match ra.address {
                    Address::ChannelAddress(c) => {
                        assert_eq!(c, 0x00010203);
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
        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)), 0x8090);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(0x00010203);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        route.addresses.push(router_address);

        let mut v: Vec<u8> = vec![];
        Route::encode(route, &mut v);
        assert_eq!(
            v,
            vec![
                3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90, 0x80, 129, 4, 3,
                2, 1, 0
            ]
        );
        match Route::decode(&v) {
            Ok((r, u)) => {
                assert_eq!(r.addresses.len(), 3);

                match r.addresses[0].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, r.addresses[0].length);
                        match r.addresses[0].address {
                            Address::UdpAddress(ip, port) => {
                                assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
                                assert_eq!(0x8080, port);
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
                            Address::UdpAddress(ip, port) => {
                                assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)));
                                assert_eq!(0x8090, port);
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
                        match r.addresses[2].address {
                            Address::ChannelAddress(a) => {
                                assert_eq!(a, 0x00010203);
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
        u16::encode(n, &mut u);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0], 0x7f);
        match u16::decode(&u) {
            Ok((m, v)) => {
                assert_eq!(u[0], 0x7f);
                assert_eq!(v.len(), 0);
            }
            Err(s) => panic!(),
        }

        let mut too_big: u16 = 0xC000;
        let mut u: Vec<u8> = vec![];
        match u16::encode(too_big, &mut u) {
            Ok(()) => panic!(),
            Err(s) => {}
        }

        let mut n = 0x80;
        let mut u: Vec<u8> = vec![];
        u16::encode(n, &mut u);
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
        u16::encode(n, &mut u);
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
        u16::encode(n, &mut u);
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
        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)), 0x8090);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(0x00010203);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        onward_route.addresses.push(router_address);

        let mut return_route = Route { addresses: vec![] };
        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let mut udp_address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)), 0x8090);
        let mut router_address = RouterAddress {
            a_type: AddressType::Udp,
            length: 0,
            address: udp_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let mut channel_address = Address::ChannelAddress(0x00010203);
        let mut router_address = RouterAddress {
            a_type: AddressType::Channel,
            length: 0,
            address: channel_address,
        };
        router_address.length = router_address.size_of();
        return_route.addresses.push(router_address);

        let mut message_body = vec![0];
        let mut msg = Box::new(Message {
            onward_route,
            return_route,
            message_body,
        });
        let mut u: Vec<u8> = vec![];
        Message::encode(msg, &mut u);
        assert_eq!(
            u,
            vec![
                3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90, 0x80, 129, 4, 3,
                2, 1, 0, 3, 2, 7, 0, 127, 0, 0, 1, 0x80, 0x80, 2, 7, 0, 10, 0, 1, 10, 0x90, 0x80,
                129, 4, 3, 2, 1, 0, 0,
            ]
        );

        match Message::decode(&u) {
            Ok((m, v)) => {
                assert_eq!(m.onward_route.addresses.len(), 3);

                match m.onward_route.addresses[0].a_type {
                    AddressType::Udp => {
                        assert_eq!(7, m.onward_route.addresses[0].length);
                        match m.onward_route.addresses[0].address {
                            Address::UdpAddress(ip, port) => {
                                assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
                                assert_eq!(0x8080, port);
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
                            Address::UdpAddress(ip, port) => {
                                assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)));
                                assert_eq!(0x8090, port);
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
                        match m.onward_route.addresses[2].address {
                            Address::ChannelAddress(a) => {
                                assert_eq!(a, 0x00010203);
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
                assert_eq!(m.message_body[0], 0);
            }
            _ => {}
        }
    }
}
