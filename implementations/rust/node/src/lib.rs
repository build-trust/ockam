extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;

use ockam_message::message::{AddressType};

use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam_message_router::MessageRouter;
use ockam_no_std_traits::{ProcessMessageHandle, PollHandle, Poll};
use std::thread;
use ockam_worker_manager::WorkerManager;

pub struct Node {
    message_router: Rc<RefCell<MessageRouter>>,
    worker_manager: Rc<RefCell<WorkerManager>>,
    modules_to_poll: VecDeque<PollHandle>,
}

impl Node {
    pub fn new() -> Result<Self, String> {
        Ok(Node {
            message_router: Rc::new(RefCell::new(MessageRouter::new().unwrap())),
            worker_manager: Rc::new(RefCell::new(WorkerManager::new())),
            modules_to_poll: VecDeque::new()})
    }

    pub fn register_worker(&mut self,
                           address: String,
                           message_handler: Option<ProcessMessageHandle>,
                           poll_handler: Option<PollHandle>) -> Result<bool, String> {

        let mut wm = self.worker_manager.deref().borrow_mut();
        wm.register_worker(address, message_handler, poll_handler)
    }

    pub fn run(&mut self) -> Result<(), String> {

        self.modules_to_poll.push_back(self.message_router.clone());

        {
            let mut mr = self.message_router.clone();
            let mut mr = mr.deref().borrow_mut();
            mr.register_address_type_handler(AddressType::Worker, self.worker_manager.clone())?;
        }
        self.modules_to_poll.push_back(self.worker_manager.clone());

        let mut stop = false;
        loop {
            for p_ref in self.modules_to_poll.iter() {
                let mut p = p_ref.deref().borrow_mut();
                match p.poll(self.message_router.clone()) {
                    Ok(keep_going) => {
                        if !keep_going {
                            stop = true;
                            break;
                        }
                    }
                    Err(s) => {
                        return Err(s);
                    }
                }
            }
            if stop { break; }
            thread::sleep(time::Duration::from_millis(500));
        }
        Ok(())
    }
}
