#![feature(refcell_take)]
#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::Deref;

#[macro_export]
macro_rules! ockam_lock_new {
    ($x:ty, $y:expr) => {{
        let rcl: Rc<RefCell<$x>> = Rc::new(RefCell::new($y));
        rcl
    }};
}

macro_rules! ockam_lock_acquire {
    ($y:expr) => {
        $y.deref().borrow_mut()
    };
}
async fn single_thread() -> u32 {
    let data1 = ockam_lock_new!(u32, 0);
    let f = async {
        let data2 = data1.clone();
        let data3 = data1.clone();
        let data4 = data1.clone();

        {
            let mut lock = ockam_lock_acquire!(data2);
            *lock += 5;
        }

        {
            let mut lock = ockam_lock_acquire!(data3);
            *lock += 5;
        }

        let mut lock = ockam_lock_acquire!(data4);
        *lock += 1;
        assert_eq!(*lock, 11);
    };
    f.await;
    let d = *data1.deref().borrow_mut();
    d
}

#[cfg(test)]
mod test {
    use crate::single_thread;
    use futures::executor::block_on;

    #[test]
    fn t1() {
        let test = single_thread();
        block_on(test);
    }
}
