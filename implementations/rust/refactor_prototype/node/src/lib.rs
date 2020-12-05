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
use ockam_print_worker::PrintWorker;

pub struct Node {}

impl Node {
    pub fn new() -> Result<Self, String> {
        Ok(Node {})
    }

    pub fn run(&mut self) -> Result<bool, String> {

        // 1. Create a queue of poll traits for anything that wants to be polled
        let mut modules_to_poll: VecDeque<Rc<RefCell<dyn Poll>>> = VecDeque::new();

        // 2. Create the message router and get the Enqueue trait, which is used
        //    by workers and message handlers to queue up any Messages they generate
        let mut message_router = MessageRouter::new().unwrap();
        let (q, mut message_router) = message_router.get_enqueue_trait();

        // 3. Create the worker manager, register its as an address-type handler, and add it to the queue of things to poll.
        //    The worker manager is responsible for routing messages of AddressType::Worker
        //    and polling workers that register for it.
        let mut worker_manager = Rc::new(RefCell::new(WorkerManager::new()));
        modules_to_poll.push_back(worker_manager.clone());
        message_router.register_address_type_handler(AddressType::Worker, worker_manager.clone());

        // 4. Now create the worker(s) and register them with the worker manager
        // ToDo: move worker creation out of node
        let mut print_worker =
            Rc::new(RefCell::new(PrintWorker::new("aabbccdd".into(), "text".into())));

        // This scoping is required so the borrow of the workers is released
        {
            let wm = worker_manager.clone();
            let mut wm = wm.deref().borrow_mut();
            wm.register_worker("aabbccdd".into(), Some(print_worker.clone()), Some(print_worker.clone()));
        }

        loop {
            for p_ref in modules_to_poll.iter() {
                let mut p = p_ref.deref().borrow_mut();
                p.poll(q.clone());
            }
            message_router.poll(q.clone());
            thread::sleep(time::Duration::from_millis(3000));
        }

        Ok(true)
    }
}
