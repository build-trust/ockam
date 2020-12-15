extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;

use ockam_message::message::{AddressType, Message};

use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam_message_router::MessageRouter;
use ockam_no_std_traits::{PollHandle, ProcessMessageHandle};
use ockam_queue::Queue;
use ockam_tcp_manager::tcp_manager::TcpManager;
use ockam_worker_manager::WorkerManager;
use std::thread;

pub struct Node {
    message_queue: Rc<RefCell<Queue<Message>>>,
    message_router: MessageRouter,
    worker_manager: Rc<RefCell<WorkerManager>>,
    modules_to_poll: VecDeque<PollHandle>,
    _role: String,
}

impl Node {
    pub fn new(role: &str) -> Result<Self, String> {
        Ok(Node {
            message_queue: Rc::new(RefCell::new(Queue::new())),
            message_router: MessageRouter::new().unwrap(),
            worker_manager: Rc::new(RefCell::new(WorkerManager::new())),
            modules_to_poll: VecDeque::new(),
            _role: role.to_string(),
        })
    }

    pub fn initialize_transport(&mut self, listen_address: Option<&str>) -> Result<bool, String> {
        let tcp_transport = TcpManager::new(listen_address)?;
        let tcp_transport = Rc::new(RefCell::new(tcp_transport));
        self.message_router
            .register_address_type_handler(AddressType::Tcp, tcp_transport.clone())?;
        self.modules_to_poll.push_back(tcp_transport);
        Ok(true)
    }

    pub fn register_worker(
        &mut self,
        address: String,
        message_handler: Option<ProcessMessageHandle>,
        poll_handler: Option<PollHandle>,
    ) -> Result<bool, String> {
        let mut wm = self.worker_manager.deref().borrow_mut();
        wm.register_worker(address, message_handler, poll_handler)
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.message_router
            .register_address_type_handler(AddressType::Worker, self.worker_manager.clone())?;
        self.modules_to_poll.push_back(self.worker_manager.clone());

        let mut stop = false;
        loop {
            match self.message_router.poll(self.message_queue.clone()) {
                Ok(keep_going) => {
                    if !keep_going {
                        break;
                    }
                }
                Err(s) => {
                    return Err(s);
                }
            }
            for p_ref in self.modules_to_poll.iter() {
                let p = p_ref.clone();
                let mut p = p.deref().borrow_mut();
                match p.poll(self.message_queue.clone()) {
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
            if stop {
                break;
            }
            thread::sleep(time::Duration::from_millis(100));
        }
        Ok(())
    }
}
