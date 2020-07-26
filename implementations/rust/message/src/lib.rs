#![allow(unused)]

// Definition and implementation of an Ockam message and message components.
// Each message component, and the message overall, implements the "Codec" trait
// allowing it to be encoded/decoded for transmission over a transport.

pub mod message {
  use std::convert::{Into, TryFrom};
  use std::error::Error;
  use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
  use std::slice;

  pub trait Codec<T> {
    fn encode(t: &mut T, v: &mut Vec<u8>);
    fn decode(s: &[u8]) -> Result<(T, &[u8]), String>;
  }

  #[derive(Debug)]
  pub struct Message {
    pub version: WireProtocolVersion,
    pub onward_route: Route,
    pub return_route: Route,
    pub message_body: MessageBody,
  }

  impl Default for Message {
    fn default() -> Message {
      Message { version: WireProtocolVersion {v: 1},
        onward_route: Route { addresses: vec![] },
        return_route: Route { addresses: vec![] },
        message_body: MessageBody::Ping }
    }
  }

  impl Codec<Message> for Message {
    fn encode(msg: &mut Message, u: &mut Vec<u8>) {
      WireProtocolVersion::encode(&mut msg.version, u);
      Route::encode(&mut msg.onward_route, u);
      Route::encode(&mut msg.return_route, u);
      MessageBody::encode(&mut msg.message_body, u);
    }
    fn decode(u: &[u8]) -> Result<(Message, &[u8]), String> {
      let mut msg: Message = Message::default();
      let mut w = u;
      match WireProtocolVersion::decode(w) {
        Ok((v, u1)) => {
          msg.version = v;
          w = u1;
        },
        Err(s) => { return Err(s); }
      }
      match Route::decode(w) {
        Ok((r, u1)) => {
          msg.onward_route = r;
          w = u1;
        },
        Err(s) => { return Err(s); }
      }
      match Route::decode(w) {
        Ok((r, u1)) => {
          msg.return_route = r;
          w = u1;
        },
        Err(s) => { return Err(s); }
      }
      match MessageBody::decode(w) {
        Ok((mb, u1)) => {
          msg.message_body = mb;
          w = u1;
        }
        Err(s) => { return Err(s); }
      }
      Ok((msg, w))
    }
  }

  /* Addresses */
  enum AddressType {
    Local = 0,
    Tcp = 1,
    Udp = 2,
  }

  #[derive(PartialEq)]
  #[derive(Debug)]
  pub struct LocalAddress {
    pub length: u8,
    pub address: Vec<u8>,
  }

  #[derive(PartialEq)]
  #[derive(Debug)]
  pub enum Address {
    LocalAddress(LocalAddress),
    TcpAddress(IpAddr, u16),
    UdpAddress(IpAddr, u16),
  }

  enum HostAddressType {
    Ipv4 = 0,
    Ipv6 = 1,
  }

  impl TryFrom<u8> for HostAddressType {
    type Error = String;
    fn try_from(data: u8) -> Result<Self, Self::Error> {
      match data {
        0 => Ok(HostAddressType::Ipv4),
        1 => Ok(HostAddressType::Ipv6),
        _ => Err("Unknown host address type".to_string())
      }
    }
  }

  impl TryFrom<u8> for AddressType {
    type Error = String;
    fn try_from(data: u8) -> Result<Self, Self::Error> {
      match data {
        0 => Ok(AddressType::Local),
        1 => Ok(AddressType::Tcp),
        2 => Ok(AddressType::Udp),
        _ => Err("Unknown address type".to_string())
      }
    }
  }

  impl Codec<Address> for Address {
    fn encode(a: &mut Address, v: &mut Vec<u8>) {
      match a {
        Address::LocalAddress(a) => {
          v.push(AddressType::Local as u8);
          LocalAddress::encode(a, v);
        },
        Address::UdpAddress(ipa, mut port) => {
          v.push(AddressType::Udp as u8);
          IpAddr::encode(ipa, v);
          v.append(&mut port.to_le_bytes().to_vec());
        },
        Address::TcpAddress(ipa, mut port) => {
          v.push(AddressType::Tcp as u8);
          IpAddr::encode(ipa, v);
          v.append(&mut port.to_le_bytes().to_vec());
        },
      }
    }
    fn decode(u: &[u8]) -> Result<(Address, &[u8]), String> {
      match (AddressType::try_from(u[0])?, &u[1..]) {
        (AddressType::Local, addr ) => {
          let (la, v) = LocalAddress::decode(addr)?;
          let address = Address::LocalAddress(la);
          Ok((address, v))
        },
        (AddressType::Tcp, addr) => { Err("Not Implemented".to_string()) },
        (AddressType::Udp, addr ) => {
          let (ipa, v) = IpAddr::decode(addr)?;
          let port = u16::from_le_bytes([v[0], v[1]]);
          let address = Address::UdpAddress(ipa, port);
          Ok((address, &v[2..]))
        }
      }
    }
  }

  impl Codec<IpAddr> for IpAddr {
    fn encode(ip: &mut IpAddr, v: &mut Vec<u8>) {
      match ip {
        std::net::IpAddr::V4(ip4) => {
          v.push( HostAddressType::Ipv4 as u8);
          v.extend_from_slice( ip4.octets().as_ref());
        },
        std::net::IpAddr::V6(ip6) => {
          v.push(HostAddressType::Ipv6 as u8);
          v.extend_from_slice( ip6.octets().as_ref());
        },
      }
    }
    fn decode(u: &[u8]) -> Result<(IpAddr, &[u8]), String> {
      match (HostAddressType::try_from(u[0])?, &u[1..]) {
        (HostAddressType::Ipv4, addr) => {
          let ip4 = Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]);
          Ok((IpAddr::V4(ip4), &u[5..]))
        },
        _ => {
          Err("".to_string())
        }
      }
    }
  }

  impl Codec<LocalAddress> for LocalAddress {
    fn encode(la: &mut LocalAddress, u: &mut Vec<u8>) {
      u.push(la.length);
      u.append(&mut la.address);
    }
    fn decode(u: &[u8]) -> Result<(LocalAddress, &[u8]), String> {
      let length =  u[0] as usize;
      let address =  u[1..length+1].to_vec();
      Ok((LocalAddress { length: u[0], address }, &u[length+1..]))
    }
  }

  /* Routes */
  #[derive(PartialEq)]
  #[derive(Debug)]
  pub struct Route {
    pub addresses: Vec<Address>,
  }

  impl Codec<Route> for Route {
    fn encode(route: &mut Route, u: &mut Vec<u8>) {
      if route.addresses.len() == 0 {
        u.push(0 as u8)
      } else {
        u.push(route.addresses.len() as u8);
        for i in (0..route.addresses.len()) {
          Address::encode(&mut route.addresses[i], u);
        }
      }
    }
    fn decode(u: &[u8]) -> Result<(Route, &[u8]), String> {
      let mut route = Route{ addresses: vec![]};
      let mut w = &u[1..];
      if 0 == u[0] { return Ok((route, &u[1..])); }
      for i in 0..u[0] as usize {
        match Address::decode(w)  {
          Ok((a, x))  => {
            route.addresses.push(a);
            w = x;
          }
          Err(s) => {
            ()
          }
        }
      }
      Ok((route, w))
    }
  }

  #[derive(Debug)]
  pub enum MessageBody {
    Ping = 0,
    Pong = 1,
    Payload = 2,
  }

  impl Default for MessageBody {
    fn default() -> MessageBody {
      MessageBody::Payload
    }
  }

  impl Codec<MessageBody> for MessageBody {
    fn encode(msg_body: &mut MessageBody, u: &mut Vec<u8>) {
      match msg_body {
        MessageBody::Ping => { u.push(MessageBody::Ping as u8); },
        MessageBody::Pong => { u.push(MessageBody::Pong as u8); },
        MessageBody::Payload => {}
      }
    }
    fn decode(u: &[u8]) -> Result<(MessageBody, &[u8]), String> {
      match MessageBody::try_from(u[0])? {
        MessageBody::Ping => { Ok((MessageBody::Ping, &u[1..])) },
        MessageBody::Pong => { Ok((MessageBody::Pong, &u[1..])) },
        _ => Err("Not implemented".to_string())
      }
    }
  }

  impl TryFrom<u8> for MessageBody {
    type Error = String;
    fn try_from(data: u8) -> Result<Self, Self::Error> {
      match data {
        0 => Ok(MessageBody::Ping),
        1 => Ok(MessageBody::Pong),
        _ => Err("Not Implemented".to_string())
      }
    }
  }

  #[allow(arithmetic_overflow)]
  impl Codec<u16> for u16 {
    fn encode(ul2: &mut u16,  u: &mut Vec<u8>) {
      if ul2 >= &mut 0xC000 { panic!() }
      let mut bytes = ul2.to_le_bytes();

      if ul2 < &mut 0x80 {
        u.push(bytes[0])
      } else {
        bytes[1] = (bytes[1] << 0x01);
        if 0 != (bytes[0] & 0x80) { bytes[1] = bytes[1] | 0x01; }
        bytes[0] |= 0x80;
        u.push(bytes[0]);
        u.push(bytes[1])
      }
    }

    #[allow(arithmetic_overflow)]
    fn decode(u: &[u8]) -> Result<(u16, &[u8]), String> {
      let mut bytes = [0,0];
      let mut ul2: u16 = 0;
      let mut i = 1;

      bytes[0] = u[0] & 0x7f;
      if (u[0] & 0x80) == 0x80 as u8 {
        bytes[0] += (u[1] & 0x01) << 7;
        bytes[1] = u[1] >>1;
        i = 2;
      }
      ul2 = ((bytes[1] as u16) <<8) + bytes[0] as u16;

      Ok((ul2, &u[i..]))
    }
  }

  #[derive(Debug)]
  pub struct WireProtocolVersion {
    pub(crate) v: u16,
  }

  impl Default for WireProtocolVersion {
    fn default() -> WireProtocolVersion {
      WireProtocolVersion{ v: 1 }
    }
  }

  impl Codec<WireProtocolVersion> for WireProtocolVersion {
    fn encode(version: &mut WireProtocolVersion, v: &mut Vec<u8>) {
      u16::encode(&mut version.v, v);
    }
    fn decode(v: &[u8]) -> Result<(WireProtocolVersion, &[u8]), String> {
      let (version, v) =  u16::decode(v)?;
      Ok((WireProtocolVersion{v: version}, v))
    }
  }

}



#[cfg(test)]
mod tests {
  use super::*;
  use std::net::{IpAddr, Ipv4Addr};
  use crate::message::*;

  #[test]
  fn local_address_codec () {
    let mut local_in = LocalAddress {length:4, address:vec![0,1,2,3]};
    let mut v: Vec<u8> = vec![];
    LocalAddress::encode(&mut local_in, &mut v);
    assert_eq!(v, [4,0,1,2,3]);
    match LocalAddress::decode(&v) {
      Ok((local_out, w)) =>
        { assert_eq!(local_out, LocalAddress {length:4, address:vec![0,1,2,3]}) },
      Err(s) => { println!("{:?}", s); }
      }
    }

  #[test]
  fn ip4_address_codec() {
    let mut v: Vec<u8> = vec![];
    let mut ip4a: IpAddr = IpAddr::V4(Ipv4Addr::new(127,0,0,1));
    IpAddr::encode(&mut ip4a, &mut v);
    assert_eq!(v, vec![0,127,0,0,1]);
    let mut v: Vec<u8> = vec![0,127,0,0,1];
    match IpAddr::decode(&v) {
      Ok((ip4a, w)) => {
        assert_eq!(ip4a, IpAddr::V4(Ipv4Addr::new(127,0,0,1)));
      }
      Err(s) => { println!("{}", s); }
    }
  }

  #[test]
  fn address_codec() {
    let mut address = Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 0x8080);
    let mut v: Vec<u8> = vec![];
    Address::encode(&mut address, &mut v);
    assert_eq!(v, vec![2,0,127,0,0,1,0x80,0x80]);
    let mut v = vec![2,0,127,0,0,1,0x80,0x80];
    match Address::decode(&mut v) {
      Ok((address, w)) => {
        assert_eq!(address, Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 0x8080));
      }
      Err(s) => { println!("{}", s); }
    }
    let mut address = Address::LocalAddress( LocalAddress{ length: 4, address:vec![0,1,2,3] } );
    let mut v: Vec<u8> = vec![];
    Address::encode(&mut address, &mut v);
    assert_eq!(v, vec![0,4,0,1,2,3]);
    let mut v = vec![0,4,0,1,2,3];
    match Address::decode(&mut v) {
      Ok((address, w)) => {
        assert_eq!(address, Address::LocalAddress(LocalAddress{ length: 4, address:vec![0,1,2,3] }));
      }
      Err(s) => { println!("{}", s); }
    }
  }

  #[test]
  fn route_codec() {
    let mut route: Route = Route{ addresses:vec![] };
    route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 0x8080));
    route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10,0,1,10)), 0x7070));
    route.addresses.push(Address::LocalAddress( LocalAddress{ length: 4, address:vec![0,1,2,3] } ));
    let mut v: Vec<u8> = vec![];
    Route::encode(&mut route, &mut v);
    assert_eq!(v, vec![3,2,0,127,0,0,1,0x80,0x80,2,0,10,0,1,10,0x70,0x70,0,4,0,1,2,3]);
    match Route::decode(&v) {
      Ok((r, u)) => {
        assert_eq!(r.addresses.len(), 3);
        assert_eq!(r.addresses[0], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), 0x8080));
        assert_eq!(r.addresses[1], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10,0,1,10)), 0x7070));
        assert_eq!(r.addresses[2], Address::LocalAddress( LocalAddress{ length: 4, address:vec![0,1,2,3] } ));
        assert_eq!(v.len(), 23);
      },
      Err(s) => { panic!(); }
    }
    let mut route: Route = Route{ addresses:vec![]};
    let mut v: Vec<u8> = vec![];
    Route::encode(&mut route, &mut v);
    assert_eq!(v.len(), 1);
    match Route::decode(&v) {
      Ok((r, v)) => {
        assert_eq!(r.addresses.len(), 0);
        assert_eq!(v.len(), 0);
      },
      Err(s) => { panic!(); }
    }

  }

  #[test]
  #[should_panic]
  fn u16_codec_too_big() {
    let mut u: Vec<u8> = vec![];
    let mut n: u16 = 0xC000;
    u16::encode(&mut n, &mut u);
  }

  #[test]
  fn u16_codec() {
    let mut u: Vec<u8> = vec![];
    let mut n: u16 = 0x7f;
    u16::encode( &mut n, &mut u);
    assert_eq!(u.len(), 1);
    assert_eq!(u[0], 0x7f);
    match u16::decode(&u) {
      Ok((m,v)) => {
        assert_eq!(u[0], 0x7f);
        assert_eq!(v.len(), 0);
      }
      Err(s) => { panic!() }
    }

    let mut n = 0x80;
    let mut u: Vec<u8> = vec![];
    u16::encode( &mut n, &mut u);
    assert_eq!(u.len(), 2);
    assert_eq!(u[0], 0x80);
    assert_eq!(u[1], 0x01);
    match u16::decode(&u[0..])  {
      Ok((m,v)) => {
        assert_eq!(m, 0x80);
        assert_eq!(v.len(), 0);
      },
      Err(e) => { panic!() }
    }

    let mut n = 0x1300;
    let mut u: Vec<u8> = vec![];
    u16::encode(&mut n, &mut u);
    assert_eq!(u.len(), 2);
    assert_eq!(u[1], 0x13<<1);
    assert_eq!(u[0], 0x80);
    match u16::decode(&u[0..])  {
      Ok((m,v)) => {
        assert_eq!(m, 0x1300);
        assert_eq!(v.len(), 0);
      },
      Err(e) => { panic!() }
    }


    let mut n = 0x1381;
    let mut u: Vec<u8> = vec![];
    u16::encode(&mut n, &mut u);
    assert_eq!(u.len(), 2);
    assert_eq!(u[1], (0x13<<1) | 1);
    assert_eq!(u[0], 0x81);
    match u16::decode(&u[0..])  {
      Ok((m,v)) => {
        assert_eq!(m, 0x1381);
        assert_eq!(v.len(), 0);
      },
      Err(e) => { panic!() }
    }
  }

  #[test]
  fn message_codec() {
    let mut onward_route: Route = Route{ addresses:vec![] };
    onward_route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080));
    onward_route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)), 0x7070));
    onward_route.addresses.push(Address::LocalAddress( LocalAddress{ length: 4, address:vec![0, 1, 2, 9] } ));
    let mut return_route: Route = Route{ addresses:vec![] };
    return_route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 0x8080));
    return_route.addresses.push(Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)), 0x7070));
    return_route.addresses.push(Address::LocalAddress( LocalAddress{ length: 4, address:vec![1, 2, 3, 4] } ));
    let mut message_body = MessageBody::Ping;
    let mut msg = Message { version: WireProtocolVersion{ v: 1 },
                        onward_route,
                        return_route,
                        message_body };
    let mut u:  Vec<u8> = vec![];
    Message::encode(&mut msg, &mut u);
    assert_eq!(u, vec![1,3,2,0,127,0,0,1,0x80,0x80,2,0,10,0,1,10,0x70,0x70,0,4,0,1,2,9,
                         3,2,0,127,0,0,2,0x80,0x80,2,0,10,0,1,11,0x70,0x70,0,4,1,2,3,4,
                         0]);

    match Message::decode(&u) {
      Ok((m,v)) => {
        assert_eq!(m.version.v, 1);
        assert_eq!(m.onward_route.addresses.len(), 3);
        assert_eq!(m.onward_route.addresses[0], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0x8080));
        assert_eq!(m.onward_route.addresses[1], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)), 0x7070));
        assert_eq!(m.onward_route.addresses[2], Address::LocalAddress( LocalAddress{ length: 4, address:vec![0, 1, 2, 9] } ));
        assert_eq!(m.return_route.addresses.len(), 3);
        assert_eq!(m.return_route.addresses[0], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 0x8080));
        assert_eq!(m.return_route.addresses[1], Address::UdpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)), 0x7070));
        assert_eq!(m.return_route.addresses[2], Address::LocalAddress( LocalAddress{ length: 4, address:vec![1, 2, 3, 4] } ));
        match m.message_body {
          MessageBody::Ping => {}
          (_) => panic!()
        }
      }
      Err(e) => { panic!() }
    }
  }
}