#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::Deref;
use ockam_message::message::{Address, AddressType, Codec, Message, RouterAddress};
use ockam_no_std_traits::{RouteMessage, HandleMessage};

pub struct TcpTransport {
    address: String
}

impl HandleMessage for TcpTransport {
    fn handle_message(
        &mut self,
        message: Message,
        q_ref: Rc<RefCell<dyn RouteMessage<Message>>>,
    ) -> Result<bool, String> {
        Ok(true)
    }
}

impl TcpTransport {
    pub fn new(address: String) -> Self {
        TcpTransport{ address }
    }
}
