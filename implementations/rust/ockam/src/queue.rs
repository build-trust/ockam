use crate::address::Addressable;
use alloc::collections::VecDeque;

pub trait Queue<T> {
    fn enqueue(&mut self, element: T) -> crate::Result<bool>;
    fn dequeue(&mut self) -> Option<T>;
    fn is_empty(&self) -> bool;
}

impl<T> Queue<T> for VecDeque<T> {
    fn enqueue(&mut self, element: T) -> crate::Result<bool> {
        self.push_back(element);
        Ok(true)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

pub fn new_queue<T: 'static>() -> impl Queue<T> {
    VecDeque::<T>::new()
}

pub trait AddressableQueue<T>: Queue<T> + Addressable {}

#[cfg(test)]
mod test {
    use crate::queue::{new_queue, Queue};

    #[test]
    fn test_queue() {
        struct Item;

        let mut queue = new_queue();

        queue.enqueue(Item {}).unwrap();
        queue.dequeue().unwrap();
    }
}
