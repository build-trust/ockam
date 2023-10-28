//! Similarly to [`ApplicationState`] the notification structure has been implemented in rust
//! and C. The idea is to work using rust structure from within the library and convert into C
//! when needed.

#[repr(C)]
#[allow(dead_code)]
pub enum Kind {
    Information = 0,
    Warning = 1,
    Error = 2,
}

pub mod rust {
    pub use crate::api::notification::Kind;

    pub struct Notification {
        pub(crate) kind: Kind,
        pub(crate) title: String,
        pub(crate) message: String,
    }

    /// Sends a notification to the application.
    #[derive(Clone)]
    pub struct NotificationCallback(
        unsafe extern "C" fn(notification: super::c::Notification) -> (),
    );
    impl NotificationCallback {
        pub fn new(
            callback: unsafe extern "C" fn(notification: super::c::Notification) -> (),
        ) -> Self {
            Self(callback)
        }
        pub fn call(&self, notification: Notification) {
            unsafe {
                (self.0)(super::convert_to_c(notification));
            }
        }
    }
}

pub(super) mod c {
    pub use crate::api::notification::Kind;
    use libc::c_char;

    #[repr(C)]
    pub struct Notification {
        pub(super) kind: Kind,
        pub(super) title: *const c_char,
        pub(super) message: *const c_char,
    }
}

use crate::api::to_c_string;

/// Convert the instance into c representation.
/// Manual call to [free] must be performed to reclaim memory.
fn convert_to_c(notification: rust::Notification) -> c::Notification {
    c::Notification {
        kind: notification.kind,
        title: to_c_string(notification.title),
        message: to_c_string(notification.message),
    }
}
