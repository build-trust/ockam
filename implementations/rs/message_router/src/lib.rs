#![allow(unused)]
#![no_std]
extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use libc_print::*;
use ockam::message::{AddressType, Message};
use ockam_no_std_traits::{EnqueueMessage, Poll, ProcessMessage, ProcessMessageHandle};
use ockam_queue::Queue;

pub struct MessageRouter {
    handlers: [Option<ProcessMessageHandle>; 256],
}

const INIT_TO_NO_RECORD: Option<ProcessMessageHandle> = None;

impl MessageRouter {
    pub fn new() -> Result<Self, String> {
        Ok(MessageRouter {
            handlers: [INIT_TO_NO_RECORD; 256],
        })
    }

    pub fn register_address_type_handler(
        &mut self,
        address_type: AddressType,
        handler: ProcessMessageHandle,
    ) -> Result<bool, String> {
        self.handlers[address_type as usize] = Some(handler);
        Ok(true)
    }

    pub fn poll(
        &self,
        mut enqueue_message_ref: Rc<RefCell<Queue<Message>>>,
    ) -> Result<bool, String> {
        loop {
            {
                let message: Option<Message> = {
                    let mut q = enqueue_message_ref.clone();
                    let mut q = q.deref().borrow_mut();
                    q.queue.remove(0)
                };
                match message {
                    Some(m) => {
                        let address_type = m.onward_route.addresses[0].a_type as usize;
                        match &self.handlers[address_type] {
                            Some(h) => {
                                let handler = h.clone();
                                let mut handler = handler.deref().borrow_mut();
                                match handler.process_message(m, enqueue_message_ref.clone()) {
                                    Ok(keep_going) => {
                                        if !keep_going {
                                            return Ok(false);
                                        }
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
                    None => {
                        break;
                    }
                }
            }
        }
        Ok(true)
    }
}
