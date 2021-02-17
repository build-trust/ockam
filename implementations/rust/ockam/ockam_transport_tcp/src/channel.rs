use crate::{Connection, TransportError};
use core::sync::atomic::AtomicI32;
use rand::prelude::*;

// pub struct Channel {
//     clear_address: Vec<u8>,
//     cipher_address: Vec<u8>,
//     serializer: Serializer,
// }
//
// impl Channel {
//     pub fn new(serializer: Serializer) -> Self {
//         let mut rng = rand::thread_rng();
//         let y: f64 = rng.gen(); // generates a float between 0 and 1
//
//         Channel { serializer }
//     }
// }

// impl Connection for Channel {
//     async fn connect(&mut self) -> Result<(), TransportError> {
//         // do the key exchange
//         unimplemented!()
//     }
//
//     async fn send(&mut self, buff: &[u8]) -> Result<usize, TransportError> {
//         unimplemented!()
//     }
//
//     async fn receive(&mut self, buff: &mut [u8]) -> Result<usize, TransportError> {
//         unimplemented!()
//     }
// }
