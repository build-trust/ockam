use crate::TransportError;
use core::sync::atomic::AtomicI32;
use ockam_key_exchange_core::CompletedKeyExchange;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use rand::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Channel {
    encrypt_addr: Vec<u8>,
    decrypt_addr: Vec<u8>,
    key: Option<CompletedKeyExchange>,
}

impl Channel {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let a: u64 = (rng.gen() * u64::pow(10, 8)) as u64;
        let encrypt_addr = a.to_le_bytes().to_vec();
        let a: u64 = (rng.gen() * u64::pow(10, 8)) as u64;
        let decrypt_addr = a.to_le_bytes().to_vec();
        let key = None;
        Self {
            encrypt_addr,
            decrypt_addr,
            key,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Channel;

    #[test]
    fn v1() {
        let channel = Channel::new();
        println!("{:?}", channel);
    }
}
