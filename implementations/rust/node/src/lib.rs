extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;

use ockam_message::message::{AddressType};

use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam_message_router::MessageRouter;
use ockam_no_std_traits::{ProcessMessageHandle, PollHandle};
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
                           message_handler: Option<ProcessMessageHandle>,
                           poll_handler: Option<PollHandle>) -> Result<bool, String> {

        let mut wm = self.worker_manager.deref().borrow_mut();
        wm.register_worker(address, message_handler, poll_handler)
    }

    pub fn run(&mut self) -> Result<(), String> {

        // 1. Create a queue of poll traits for anything that wants to be polled
        let mut modules_to_poll: VecDeque<PollHandle> = VecDeque::new();

        // 2. Create the message router and get the Enqueue trait, which is used
        //    by workers and message handlers to queue up any Messages they generate
        let mut message_router = MessageRouter::new().unwrap();
        message_router.register_address_type_handler(AddressType::Worker, self.worker_manager.clone())?;
        let (q, message_router) = message_router.get_enqueue_trait();
        let mr_ref = Rc::new(RefCell::new(message_router));
        modules_to_poll.push_back(mr_ref.clone());

        modules_to_poll.push_back(self.worker_manager.clone());

        let mut stop = false;
        loop {
            for p_ref in modules_to_poll.iter() {
                let mut p = p_ref.deref().borrow_mut();
                match p.poll(q.clone()) {
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
