use serde::{Deserialize, Serialize};
use serde_bare;
use std::net::SocketAddr;
use std::str::FromStr;
use rust_elixir_msg::message::IP4TcpAddress;
use std::convert::TryFrom;

fn main() {
    let tcp_addr = IP4TcpAddress::try_from("127.0.0.1:4050").unwrap();
    println!("{:?}", tcp_addr);
    // let v = serde_bare::to_vec("hello world").unwrap();
    // println!("{:?}", v);
    //
    // let i1 = serde_bare::Int(16);
    // let i2 = serde_bare::Int(256);
    //
    // let mut v_i1 = serde_bare::to_vec(&i1).unwrap();
    // let mut v_i2 = serde_bare::to_vec(&i2).unwrap();
    //
    // println!("16: {:?}, 256: {:?}", v_i1, v_i2);
    //
    // println!("v_i1.len(): {}, v_i2.len(): {}", v_i1.len(), v_i2.len());
    //
    // v_i1.append(&mut vec![0; 7]);
    // v_i2.append(&mut vec![0; 6]);
    //
    // let i1 = serde_bare::from_slice::<serde_bare::Int>(&v_i1).unwrap();
    // let i2 = serde_bare::from_slice::<serde_bare::Int>(&v_i2).unwrap();
    //
    // println!("should be 16: {:?} should be 256: {:?}", i1, i2);
}
