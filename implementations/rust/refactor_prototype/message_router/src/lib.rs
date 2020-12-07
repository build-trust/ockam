#![no_std]
extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use libc_print::*;
use ockam_message::message::{AddressType, Message};
use ockam_no_std_traits::{RouteMessage, Poll, ProcessMessageHandle, RouteMessageHandle};

pub struct MessageRouter {
    handlers: [Option<ProcessMessageHandle>; 256],
    message_queue: Rc<RefCell<VecDeque<Message>>>,
}

const INIT_TO_NO_RECORD: Option<ProcessMessageHandle> = None;

impl MessageRouter {
    pub fn new() -> Result<Self, String> {
        Ok(MessageRouter {
            handlers: [INIT_TO_NO_RECORD; 256],
            message_queue: Rc::new(RefCell::new(VecDeque::new())),
        })
    }

    pub fn register_address_type_handler(
        &mut self,
        address_type: AddressType,
        handler: ProcessMessageHandle,
    ) -> Result<bool, String> {
        self.handlers[address_type as usize] = Some(handler);
        libc_println!("registered {:?}", address_type);
        Ok(true)
    }

    pub fn get_enqueue_trait(self) -> (RouteMessageHandle<Message>, Self) {
        (self.message_queue.clone(), self)
    }
}

impl RouteMessage<Message> for MessageRouter {
    fn route_message(&mut self, m: Message) -> Result<bool, String> {
        let mut q = self.message_queue.deref().borrow_mut();
        q.push_back(m);
        Ok(true)
    }
}

impl Poll for MessageRouter {
    fn poll(&mut self, q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
        libc_println!("in MessageRouter: Poll");
        let mut q = self.message_queue.deref().borrow_mut();
        for m in q.drain(..) {
            libc_println!("routing by address type");
            let address_type = m.onward_route.addresses[0].a_type as usize;
            match &self.handlers[address_type] {
                Some( h) => {
                    let handler = h.clone();
                    let mut handler = handler.deref().borrow_mut();
                    match handler.handle_message(m, q_ref.clone()) {
                        Ok(keep_going) => {
                            if !keep_going { return Ok(false); }
                        }
                        Err(s) => {
                            return Err(s);
                        }
                    }
                }
                None => {
                    return Err("no handler for message type".into());
                }
            }
        }
        Ok(true)
    }
}
