use std::thread;
extern crate alloc;

#[macro_export]
macro_rules! ockam_lock_new {
    ($x:ty, $y:expr) => {
        {
            let rcl: alloc::sync::Arc<std::sync::Mutex<$x>> = alloc::sync::Arc::new(std::sync::Mutex::new($y));
            rcl
        }
    };
}

macro_rules! ockam_lock_acquire {
    ($y:expr) => {
        {
            $y.lock().unwrap()
        }
    };
}

async fn test_lock() -> u32 {
    let data1 = ockam_lock_new!(u32, 0);
    let f = async {
        let data2 = data1.clone();
        let data3 = data1.clone();

        let j1 = thread::spawn( move || {
            let mut lock = ockam_lock_acquire!(data2);
            *lock += 5;
        });

        let j2 = thread::spawn( move || {
            let mut lock = ockam_lock_acquire!(data3);
            *lock += 5;
        });

        j1.join();
        j2.join();

        let mut lock = ockam_lock_acquire!(data1);
        *lock += 1;
        assert_eq!(*lock, 11);

    };
    f.await;
    let d1 = *ockam_lock_acquire!(data1);
    d1
}

#[cfg(test)]
mod test {
    use crate::test_lock;
    use futures::executor::block_on;

    #[test]
    fn tokio_multi_thread() {
        let t = test_lock();
        let n = block_on(t);
        assert_eq!(n, 11)
    }
}