#![no_std]
extern crate alloc;

#[macro_export]
macro_rules! ockam_lock_new {
    ($x:ty, $y:expr) => {{
        let rcl: alloc::sync::Arc<tokio::sync::Mutex<$x>> =
            alloc::sync::Arc::new(tokio::sync::Mutex::new($y));
        rcl
    }};
}

macro_rules! ockam_lock_acquire {
    ($y:expr) => {{
        $y.lock()
    }};
}

#[cfg(test)]
mod test {

    #[tokio::test(flavor = "current_thread")]
    async fn tokio_single_thread() {
        let f = async {
            let data1 = ockam_lock_new!(u32, 0);
            let data2 = data1.clone();
            let data3 = data1.clone();
            let data4 = data1.clone();

            let j1 = tokio::spawn(async move {
                let mut lock = ockam_lock_acquire!(data2).await;
                *lock += 5;
            });

            let j2 = tokio::spawn(async move {
                let mut lock = ockam_lock_acquire!(data3).await;
                *lock += 5;
            });

            let (r1, r2) = tokio::join!(j1, j2);
            match r1 {
                Err(_) => {
                    assert!(false);
                }
                _ => {}
            }
            match r2 {
                Err(_) => {
                    assert!(false);
                }
                _ => {}
            }

            let mut lock = ockam_lock_acquire!(data4).await;
            *lock += 1;
            assert_eq!(*lock, 11);
        };
        f.await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tokio_multi_thread() {
        let f = async {
            let data1 = ockam_lock_new!(u32, 0);
            let data2 = data1.clone();
            let data3 = data1.clone();

            let j1 = tokio::spawn(async move {
                let mut lock = ockam_lock_acquire!(data2).await;
                *lock += 5;
            });

            let j2 = tokio::spawn(async move {
                let mut lock = ockam_lock_acquire!(data3).await;
                *lock += 5;
            });

            let (r1, r2) = tokio::join!(j1, j2);
            match r1 {
                Err(_) => {
                    assert!(false);
                }
                _ => {}
            }
            match r2 {
                Err(_) => {
                    assert!(false);
                }
                _ => {}
            }

            let mut lock = ockam_lock_acquire!(data1).await;
            *lock += 1;
            assert_eq!(*lock, 11);
        };
        f.await;
    }
}
