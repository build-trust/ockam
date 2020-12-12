#![allow(unused)]
extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::Deref;
use ockam_message::message::{Address, AddressType, Codec, Message, RouterAddress};
use ockam_no_std_traits::{RouteMessage, ProcessMessage, RouteMessageHandle};

pub struct TcpTransport {
    address: String
}

impl ProcessMessage for TcpTransport {
    fn handle_message(
        &mut self,
        message: Message,
        q_ref: RouteMessageHandle<Message>,
    ) -> Result<bool, String> {
        Ok(true)
    }
}

impl TcpTransport {
    pub fn new(address: String) -> Self {
        TcpTransport{ address }
    }
}
