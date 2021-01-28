extern crate alloc;

use tokio::sync::Mutex;
use std::sync::Arc;

#[macro_export]
macro_rules! ockam_trait {
    (dyn $trait:ident $($t:tt)*) => {
        Arc<Mutex<dyn $trait $($t)*>>
     };
}

#[macro_export]
macro_rules! ockam_lock_new {
    ($x:ty, $y:expr) => {{
        let rcl: alloc::sync::Arc<tokio::sync::Mutex<$x>> =
            alloc::sync::Arc::new(tokio::sync::Mutex::new($y));
        rcl
    }};
}

#[macro_export]
macro_rules! ockam_lock_acquire {
    ($y:expr) => {{
        $y.lock().await
    }};
}

