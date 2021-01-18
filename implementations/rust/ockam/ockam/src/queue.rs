use crate::address::{Address, Addressable};
use alloc::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

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

pub trait AddressableQueue<T>: Queue<T> + Addressable {}

#[derive(Debug)]
pub struct AddressedVec<T> {
    pub address: Address,
    pub vec: VecDeque<T>,
}

impl<T> AddressedVec<T> {
    pub fn new(address: Address) -> Self {
        AddressedVec {
            address,
            vec: VecDeque::new(),
        }
    }
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

pub type QueueHandle<T> = Rc<RefCell<dyn AddressableQueue<T>>>;

pub trait Drain<T> {
    fn drain(&mut self, f: impl FnMut(T));
}

impl<T> Drain<T> for dyn AddressableQueue<T> {
    fn drain(&mut self, mut f: impl FnMut(T)) {
        while let Some(element) = self.dequeue() {
            f(element);
        }
    }
}

pub fn new_queue<T: 'static, A: Into<Address>>(address: A) -> Rc<RefCell<dyn AddressableQueue<T>>> {
    Rc::new(RefCell::new(AddressedVec::<T>::new(address.into())))
}

#[cfg(test)]
mod test {
    use crate::queue::new_queue;

    #[test]
    fn test_queue() {
        struct Item;

        let queue_handle = new_queue("test");
        let mut queue = queue_handle.borrow_mut();

        match queue.enqueue(Item {}) {
            Ok(_) => {}
            Err(_) => panic!(),
        };
        match queue.dequeue() {
            Some(_) => {}
            None => panic!(),
        };
    }
}
