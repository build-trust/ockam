use crate::access_control::AccessControl;
use crate::compat::boxed::Box;
use crate::{LocalMessage, Result};

/// An Access Control type that blocks all messages from passing through.
pub struct DenyAll;

#[async_trait]
impl AccessControl for DenyAll {
    async fn is_authorized(&self, _local_msg: &LocalMessage) -> Result<bool> {
        crate::deny()
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::Result;
    use crate::{route, LocalMessage, TransportMessage};
    use futures_util::future::{Future, FutureExt};

    use super::{AccessControl, DenyAll};

    #[test]
    fn test_deny_all() {
        let is_authorized = poll_once(async {
            let local_message =
                LocalMessage::new(TransportMessage::v1(route![], route![], vec![]), vec![]);
            DenyAll.is_authorized(&local_message).await
        });
        assert!(
            is_authorized.is_ok(),
            "this implementation should never return Err"
        );
        let is_authorized = is_authorized.ok();
        assert_eq!(is_authorized, crate::deny().ok());
        assert_ne!(is_authorized, crate::allow().ok());
    }

    /// TODO document
    /// TODO move somewhere sensible
    fn poll_once<'a, F, T>(future: F) -> Result<T>
    where
        F: Future<Output = Result<T>> + Send + 'a,
    {
        use core::task::{Context, Poll};
        use core::task::{RawWaker, RawWakerVTable, Waker};

        fn dummy_raw_waker() -> RawWaker {
            fn no_op(_: *const ()) {}
            fn clone(_: *const ()) -> RawWaker {
                dummy_raw_waker()
            }
            let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
            RawWaker::new(core::ptr::null(), vtable)
        }

        fn dummy_waker() -> Waker {
            // The RawWaker's vtable only contains safe no-op
            // functions which do not refer to the data field.
            #[allow(unsafe_code)]
            unsafe {
                Waker::from_raw(dummy_raw_waker())
            }
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        let result = future.boxed().poll_unpin(&mut context);
        assert!(
            result.is_ready(),
            "poll_once() only accepts futures that resolve after being polled once"
        );
        match result {
            Poll::Ready(value) => value,
            Poll::Pending => Err(crate::Error::new(1000, "SOME_ERROR_DOMAIN")),
        }
    }
}
