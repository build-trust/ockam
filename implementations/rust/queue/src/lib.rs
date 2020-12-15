extern crate alloc;
use alloc::collections::VecDeque;
use ockam_message::message::Message;
use ockam_no_std_traits::EnqueueMessage;

pub struct Queue<T> {
    pub queue: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            queue: VecDeque::new(),
        }
    }
}

impl EnqueueMessage for Queue<Message> {
    fn enqueue_message(&mut self, message: Message) -> Result<bool, String> {
        self.queue.push_back(message);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
