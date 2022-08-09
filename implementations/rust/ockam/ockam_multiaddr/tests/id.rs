use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::{Code, MultiAddr, Protocol};
use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::prelude::*;
use std::collections::VecDeque;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Newtype to implement `Arbitrary` for.
#[derive(Debug, Clone)]
struct Addr(MultiAddr);

quickcheck! {
    fn to_str_from_str(a: Addr) -> bool {
        a.0 == MultiAddr::try_from(a.0.to_string().as_str()).unwrap()
    }

    fn to_bytes_from_bytes(a: Addr) -> bool {
        a.0 == MultiAddr::try_from(a.0.as_ref()).unwrap()
    }

    fn push_back_value(a: Addr) -> bool {
        let mut ma = MultiAddr::default();
        for proto in a.0.iter() {
            ma.push_back_value(&proto).unwrap()
        }
        a.0 == ma
    }

    fn push_front_value(a: Addr) -> bool {
        let mut vec = Vec::new();
        for proto in a.0.iter() {
            vec.push(proto)
        }
        let mut ma = MultiAddr::default();
        for proto in vec.iter().rev() {
            ma.push_front_value(&proto).unwrap()
        }
        a.0 == ma
    }

    fn operations(ops: Vec<Op>) -> bool {
        let mut gen = rand::thread_rng();
        let mut addr = MultiAddr::default();
        let mut prot = VecDeque::new();
        for o in &ops {
            match o {
                Op::PopBack => {
                    addr.pop_back();
                    prot.pop_back();
                }
                Op::PopFront => {
                    addr.pop_front();
                    prot.pop_front();
                }
                Op::DropLast => {
                    addr.drop_last();
                    prot.pop_back();
                }
                Op::DropFirst => {
                    addr.drop_first();
                    prot.pop_front();
                }
                Op::Clone => {
                    addr = addr.clone()
                }
                Op::PushBack => match *PROTOS.choose(&mut gen).unwrap() {
                    Tcp::CODE => {
                        addr.push_back(Tcp::new(0)).unwrap();
                        prot.push_back(Tcp::CODE);
                    }
                    DnsAddr::CODE => {
                        addr.push_back(DnsAddr::new("localhost")).unwrap();
                        prot.push_back(DnsAddr::CODE);
                    }
                    Ip4::CODE => {
                        addr.push_back(Ip4::new([172,0,0,2])).unwrap();
                        prot.push_back(Ip4::CODE)
                    }
                    Ip6::CODE => {
                        addr.push_back(Ip6::new(Ipv6Addr::from_str("::1").unwrap())).unwrap();
                        prot.push_back(Ip6::CODE)
                    }
                    _ => unreachable!()
                }
            }
            if prot.iter().zip(addr.iter()).any(|(a, b)| *a != b.code()) {
                return false
            }
        }
        true
    }
}

const PROTOS: &[Code] = &[Tcp::CODE, DnsAddr::CODE, Ip4::CODE, Ip6::CODE];

impl Arbitrary for Addr {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut a = MultiAddr::default();
        for _ in 0..g.size() {
            match *g.choose(PROTOS).unwrap() {
                Tcp::CODE => a.push_back(Tcp::new(u16::arbitrary(g))).unwrap(),
                DnsAddr::CODE => a.push_back(DnsAddr::new(gen_hostname())).unwrap(),
                Ip4::CODE => a.push_back(Ip4::new(Ipv4Addr::arbitrary(g))).unwrap(),
                Ip6::CODE => a.push_back(Ip6::new(Ipv6Addr::arbitrary(g))).unwrap(),
                _ => unreachable!(),
            }
        }
        Addr(a)
    }
}

/// An operation to perform on a MultiAddr.
#[derive(Debug, Copy, Clone)]
enum Op {
    PushBack,
    PopBack,
    PopFront,
    DropLast,
    DropFirst,
    Clone,
}

impl Arbitrary for Op {
    fn arbitrary(g: &mut Gen) -> Self {
        *g.choose(&[
            Op::PushBack,
            Op::PopBack,
            Op::PopFront,
            Op::DropLast,
            Op::DropFirst,
            Op::Clone,
        ])
        .unwrap()
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
