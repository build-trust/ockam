#![no_std]
extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use ockam_message::message::{Message};

/// ProcessMessage trait is for workers to process messages addressed to them
///
/// A worker registers its address along with a ProcessMessage trait. The WorkerManager
/// will then call the ProcessMessage trait when the next onward_route address is that of
/// the worker.
pub trait ProcessMessage {
    fn handle_message(
        &mut self,
        message: Message, //todo - add context
        queue: RouteMessageHandle<Message>,
    ) -> Result<bool, String>;
}
pub type ProcessMessageHandle = Rc<RefCell<dyn ProcessMessage>>;

/// Poll trait is for workers to get cpu cycles on a regular basis.
///
/// A worker gets polled by registering its address and Poll trait with the Node.
/// poll() will be called once each polling interval.
pub trait Poll { //todo - add context
    fn poll(&mut self, q_ref: RouteMessageHandle<Message>) -> Result<bool, String>;
}
pub type PollHandle = Rc<RefCell<dyn Poll>>;

/// RouteMessage trait is used by Workers to enqueue messages for routing.
///
/// The RouteMessage trait is passed to a Worker each time it is sent a message or polled.
pub trait RouteMessage<T> {
    fn route_message(&mut self, t: T) -> Result<bool, String>;
}
pub type RouteMessageHandle<T> = Rc<RefCell<dyn RouteMessage<T>>>;

impl<T> RouteMessage<T> for VecDeque<T> {
    fn route_message(&mut self, t: T) -> Result<bool, String> {
        self.push_back(t);
        Ok(true)
    }
}
