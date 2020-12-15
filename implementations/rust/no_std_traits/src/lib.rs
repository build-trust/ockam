#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use ockam_message::message::Message;

/// ProcessMessage trait is for workers to process messages addressed to them
///
/// A worker registers its address along with a ProcessMessage trait. The WorkerManager
/// will then call the ProcessMessage trait when the next onward_route address is that of
/// the worker.
pub trait ProcessMessage {
    fn process_message(
        &mut self,
        message: Message, //todo - add context
        enqueue: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String>;
}
pub type ProcessMessageHandle = Rc<RefCell<dyn ProcessMessage>>;

/// poll trait is for workers to get cpu cycles on a regular basis.
///
/// A worker gets polled by registering its address and poll trait with the Node.
/// poll() will be called once each polling interval.
pub trait Poll {
    //todo - add context
    fn poll(&mut self, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String>;
}
pub type PollHandle = Rc<RefCell<dyn Poll>>;

pub trait EnqueueMessage {
    fn enqueue_message(&mut self, message: Message) -> Result<bool, String>;
}
