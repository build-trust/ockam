use crate::{Connection, TransportError};
use core::sync::atomic::AtomicI32;
use rand::prelude::*;

pub struct Channel {
    clear_address: Vec<u8>,
    cipher_address: Vec<u8>,
}

impl Channel {}
