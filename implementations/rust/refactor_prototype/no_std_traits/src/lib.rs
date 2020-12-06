#![no_std]
extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use core::ops::DerefMut;
use ockam_message::message::{AddressType, Message};

pub trait MessageHandler {
    fn handle_message(
        &mut self,
        message: Message,
        q_ref: Rc<RefCell<dyn Enqueue<Message>>>,
    ) -> Result<bool, String>;
}

pub trait Poll {
    fn poll(&mut self, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String>;
}

pub trait Enqueue<T> {
    fn enqueue(&mut self, t: T) -> Result<bool, String>;
}

impl<T> Enqueue<T> for VecDeque<T> {
    fn enqueue(&mut self, t: T) -> Result<bool, String> {
        self.push_back(t);
        Ok(true)
    }
}
