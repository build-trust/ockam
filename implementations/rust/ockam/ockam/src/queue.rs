use crate::address::{Address, Addressable};
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

pub struct AddressedVec<T> {
    pub(crate) address: Address,
    pub(crate) vec: VecDeque<T>,
}

impl<T> Queue<T> for AddressedVec<T> {
    fn enqueue(&mut self, element: T) -> crate::Result<bool> {
        self.vec.enqueue(element)
    }

    fn dequeue(&mut self) -> Option<T> {
        self.vec.dequeue()
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
}

impl<T> Addressable for AddressedVec<T> {
    fn address(&self) -> Address {
        self.address.clone()
    }
}

impl<T> AddressableQueue<T> for AddressedVec<T> {}

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
