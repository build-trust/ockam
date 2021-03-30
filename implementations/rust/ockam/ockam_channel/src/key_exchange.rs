use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum KeyExchangeRequestMessage {
    Payload { req_id: Vec<u8>, payload: Vec<u8> },
    InitiatorFirstMessage { req_id: Vec<u8> },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct Keys {
    h: [u8; 32],
    encrypt_key: usize,
    decrypt_key: usize,
}

impl Keys {
    pub fn new(h: [u8; 32], encrypt_key: usize, decrypt_key: usize) -> Self {
        Keys {
            h,
            encrypt_key,
            decrypt_key,
        }
    }
}

impl Keys {
    pub fn h(&self) -> [u8; 32] {
        self.h
    }
    pub fn encrypt_key(&self) -> usize {
        self.encrypt_key
    }
    pub fn decrypt_key(&self) -> usize {
        self.decrypt_key
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub(crate) struct KeyExchangeResponseMessage {
    req_id: Vec<u8>,
    payload: Option<Vec<u8>>,
    keys: Option<Keys>,
}

impl KeyExchangeResponseMessage {
    pub fn req_id(&self) -> &Vec<u8> {
        &self.req_id
    }
    pub fn payload(&self) -> &Option<Vec<u8>> {
        &self.payload
    }
    pub fn keys(&self) -> &Option<Keys> {
        &self.keys
    }
}

impl KeyExchangeResponseMessage {
    pub fn new(req_id: Vec<u8>, payload: Option<Vec<u8>>, keys: Option<Keys>) -> Self {
        KeyExchangeResponseMessage {
            req_id,
            payload,
            keys,
        }
    }
}

mod initiator;
mod responder;

pub(crate) use initiator::*;
pub(crate) use responder::*;
