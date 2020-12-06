extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;

use ockam_message::message::{AddressType, Message, MessageType, Route};

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam_message_router::MessageRouter;
use ockam_no_std_traits::{Enqueue, MessageHandler, Poll};
use ockam_tcp_manager::tcp_manager::TcpManager;
use std::thread;
use ockam_worker_manager::WorkerManager;

pub struct Node {
    worker_manager: Rc<RefCell<WorkerManager>>
}

impl Node {
    pub fn new() -> Result<Self, String> {
        Ok(Node {worker_manager: Rc::new(RefCell::new(WorkerManager::new()))})
    }

    pub fn register_worker(&mut self,
                           address: String,
                           message_handler: Option<Rc<RefCell<dyn MessageHandler>>>,
                           poll_handler: Option<Rc<RefCell<dyn Poll>>>) -> Result<bool, String> {

        let mut wm = self.worker_manager.deref().borrow_mut();
        wm.register_worker(address, message_handler, poll_handler)
    }

    pub fn run(&mut self) -> Result<(), String> {

        // 1. Create a queue of poll traits for anything that wants to be polled
        let mut modules_to_poll: VecDeque<Rc<RefCell<dyn Poll>>> = VecDeque::new();

        // 2. Create the message router and get the Enqueue trait, which is used
        //    by workers and message handlers to queue up any Messages they generate
        let mut message_router = MessageRouter::new().unwrap();
        let (q, mut message_router) = message_router.get_enqueue_trait();

        modules_to_poll.push_back(self.worker_manager.clone());
        message_router.register_address_type_handler(AddressType::Worker, self.worker_manager.clone());

        loop {
            for p_ref in modules_to_poll.iter() {
                let mut p = p_ref.deref().borrow_mut();
                match p.poll(q.clone()) {
                    Ok(keep_going) => {
                        if !keep_going { break; }
                    }
                    Err(s) => {
                        return Err(s);
                    }
                }
            }
            match message_router.poll(q.clone()) {
                Ok(keep_going) => {
                    if !keep_going { break; }
                }
                Err(s) => {
                    return Err(s);
                }
            }
            thread::sleep(time::Duration::from_millis(500));
        }
        Ok(())
    }
}
