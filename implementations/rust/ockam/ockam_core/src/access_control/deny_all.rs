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
    use crate::compat::future::poll_once;
    use crate::{route, LocalMessage, TransportMessage};

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
}
