// use crate::TransportError;
// use core::sync::atomic::AtomicI32;
// use rand::prelude::*;
// use std::sync::{Arc, Mutex};
//
// pub struct Channel {
//     clear_address: Vec<u8>,
//     cipher_address: Vec<u8>,
// }
//
// impl Channel {
//     pub fn new() -> Self {
//         Channel {
//             clear_address: vec![],
//             cipher_address: vec![],
//         }
//     }
//     pub fn initialize(&mut self) {
//         let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
//         let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
//         let key_exchanger =
//             XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());
//
//         let mut initiator = key_exchanger.initiator();
//         let mut responder = key_exchanger.responder();
//     }
// }
