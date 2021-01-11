use crate::address::{Address, Addressable};
use crate::queue::{AddressableQueue, Queue};
use crate::route::Route;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

pub type Payload = Vec<u8>;

#[derive(Debug, Copy, Clone)]
pub enum MessageType {
    Payload,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub onward_route: Route,
    pub return_route: Route,
    pub payload: Payload,
}

impl From<Payload> for Message {
    fn from(payload: Payload) -> Self {
        Message {
            message_type: MessageType::Payload,
            onward_route: Route::new(),
            return_route: Route::new(),
            payload,
        }
    }
}

impl Into<Message> for &str {
    fn into(self) -> Message {
        Message::from(self.as_bytes().to_vec())
    }
}

impl Into<Message> for i32 {
    fn into(self) -> Message {
        Message::from(self.to_le_bytes().to_vec())
    }
}

impl Message {
    pub fn empty() -> Self {
        Message::from(vec![])
    }

    pub fn onward_add(&mut self, address: Address) {
        self.onward_route.append(address.into());
    }

    pub fn return_add(&mut self, address: Address) {
        self.onward_route.append(address.into());
    }
}

pub struct MessageBuilder {
    message_type: Option<MessageType>,
    payload: Option<Payload>,
    onward_route: Route,
    return_route: Route,
}

impl MessageBuilder {
    pub fn message() -> Self {
        MessageBuilder {
            message_type: None,
            payload: None,
            onward_route: Route::new(),
            return_route: Route::new(),
        }
    }

    pub fn message_type(&mut self, message_type: MessageType) -> &mut Self {
        self.message_type = Some(message_type);
        self
    }

    pub fn payload(&mut self, payload: Payload) -> &mut Self {
        self.payload = Some(payload);
        self
    }

    pub fn empty(&mut self) -> &mut Self {
        self.payload = Some(vec![]);
        self
    }

    pub fn onward_route(&mut self, onward_route: Route) -> &mut Self {
        self.onward_route = onward_route;
        self
    }

    pub fn onward_to(&mut self, onward: &str) -> &mut Self {
        self.onward_route.append(Address::from(onward).into());
        self
    }

    pub fn return_route(&mut self, return_route: Route) -> &mut Self {
        self.return_route = return_route;
        self
    }

    pub fn return_to(&mut self, ret: &str) -> &mut Self {
        self.return_route.append(ret.into());
        self
    }

    pub fn build(&self) -> Message {
        let message_type = if let Some(t) = self.message_type {
            t
        } else {
            MessageType::Payload
        };

        let payload = if let Some(p) = &self.payload {
            p.to_vec()
        } else {
            vec![]
        };

        Message {
            message_type,
            onward_route: self.onward_route.clone(),
            return_route: self.return_route.clone(),
            payload,
        }
    }
}

struct MessageQueue {
    address: Address,
    inner: VecDeque<Message>,
}

impl MessageQueue {
    fn new(address: Address) -> Self {
        MessageQueue {
            address,
            inner: VecDeque::new(),
        }
    }
}

impl Queue<Message> for MessageQueue {
    fn enqueue(&mut self, element: Message) -> crate::Result<bool> {
        self.inner.enqueue(element)
    }

    fn dequeue(&mut self) -> Option<Message> {
        self.inner.dequeue()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Addressable for MessageQueue {
    fn address(&self) -> Address {
        self.address.clone()
    }
}

impl AddressableQueue<Message> for MessageQueue {}

pub fn new_message_queue(address: Address) -> Rc<RefCell<dyn AddressableQueue<Message>>> {
    Rc::new(RefCell::new(MessageQueue::new(address)))
}
