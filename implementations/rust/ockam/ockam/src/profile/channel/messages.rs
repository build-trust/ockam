use crate::Contact;
use ockam_core::Address;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Confirm;

#[derive(Serialize, Deserialize)]
pub(crate) struct ChannelAuthRequest {
    contact: Contact,
    proof: Vec<u8>,
}

impl ChannelAuthRequest {
    pub fn contact(&self) -> &Contact {
        &self.contact
    }
    pub fn proof(&self) -> &Vec<u8> {
        &self.proof
    }
}

impl ChannelAuthRequest {
    pub fn new(contact: Contact, proof: Vec<u8>) -> Self {
        Self { contact, proof }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChannelAuthResponse {
    contact: Contact,
    proof: Vec<u8>,
    channel_address: Address,
}

impl ChannelAuthResponse {
    pub fn contact(&self) -> &Contact {
        &self.contact
    }
    pub fn proof(&self) -> &Vec<u8> {
        &self.proof
    }
    pub fn channel_address(&self) -> &Address {
        &self.channel_address
    }
}

impl ChannelAuthResponse {
    pub fn new(contact: Contact, proof: Vec<u8>, channel_address: Address) -> Self {
        Self {
            contact,
            proof,
            channel_address,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChannelAuthConfirm {
    channel_address: Address,
}

impl ChannelAuthConfirm {
    pub fn channel_address(&self) -> &Address {
        &self.channel_address
    }
}

impl ChannelAuthConfirm {
    pub fn new(channel_address: Address) -> Self {
        Self { channel_address }
    }
}
