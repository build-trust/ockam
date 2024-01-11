use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::runtime::Runtime;

pub(crate) static RUNTIME: Lazy<Mutex<Option<Runtime>>> =
    Lazy::new(|| Mutex::new(Some(Runtime::new().unwrap())));

/// Return the Runtime singleton
/// This function can only be accessed once
pub fn take() -> Runtime {
    RUNTIME
        .lock()
        .unwrap()
        .take()
        .expect("Runtime was consumed")
}
