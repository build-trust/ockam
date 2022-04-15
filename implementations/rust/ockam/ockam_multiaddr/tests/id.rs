use ockam_multiaddr::proto::{Dns, Tcp};
use ockam_multiaddr::{Code, MultiAddr, Protocol};
use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::prelude::*;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Newtype to implement `Arbitrary` for.
#[derive(Debug, Clone)]
struct Addr(MultiAddr);

quickcheck! {
    fn to_str_from_str(a: Addr) -> bool {
        let s = a.0.to_string();
        let b = MultiAddr::try_from(s.as_str()).unwrap();
        a.0 == b
    }

    fn to_bytes_from_bytes(a: Addr) -> bool {
        let v: Vec<u8> = a.0.clone().into();
        let b = MultiAddr::try_from(v.as_slice()).unwrap();
        a.0 == b
    }
}

const PROTOS: &[Code] = &[Tcp::CODE, Dns::CODE, Ipv4Addr::CODE, Ipv6Addr::CODE];

impl Arbitrary for Addr {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut a = MultiAddr::default();
        for _ in 0..g.size() {
            match *g.choose(PROTOS).unwrap() {
                Tcp::CODE => a.push_back(Tcp(u16::arbitrary(g))).unwrap(),
                Dns::CODE => a.push_back(Dns::new(gen_hostname())).unwrap(),
                Ipv4Addr::CODE => a.push_back(Ipv4Addr::arbitrary(g)).unwrap(),
                Ipv6Addr::CODE => a.push_back(Ipv6Addr::arbitrary(g)).unwrap(),
                _ => unreachable!(),
            }
        }
        Addr(a)
    }
}

fn gen_hostname() -> String {
    const LABEL: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz123456789_-";
    fn gen_label<R: Rng>(g: &mut R) -> String {
        let num: usize = g.gen_range(1..=23);
        String::from_iter(LABEL.chars().choose_multiple(g, num).into_iter())
    }
    let mut g = rand::thread_rng();
    let mut v = Vec::new();
    for _ in 1..=10 {
        v.push(gen_label(&mut g))
    }
    v.join(".")
}
