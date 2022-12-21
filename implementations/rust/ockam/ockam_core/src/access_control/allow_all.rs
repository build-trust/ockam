use crate::access_control::IncomingAccessControl;
use crate::compat::boxed::Box;
use crate::{OutgoingAccessControl, RelayMessage, Result};

/// An Access Control type that allows all messages to pass through.
#[derive(Debug)]
pub struct AllowAll;

#[async_trait]
impl IncomingAccessControl for AllowAll {
    async fn is_authorized(&self, _relay_msg: &RelayMessage) -> Result<bool> {
        crate::allow()
    }
}

#[async_trait]
impl OutgoingAccessControl for AllowAll {
    async fn is_authorized(&self, _relay_msg: &RelayMessage) -> Result<bool> {
        crate::allow()
    }
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use crate::compat::future::poll_once;
    use crate::{route, Address, LocalMessage, RelayMessage, TransportMessage};

    use super::{AllowAll, IncomingAccessControl};

    #[test]
    fn test_allow_all() {
        let is_authorized = poll_once(async {
            let local_message =
                LocalMessage::new(TransportMessage::v1(route![], route![], vec![]), vec![]);
            let relay_message = RelayMessage::new(
                Address::random_local(),
                Address::random_local(),
                local_message,
            );
            AllowAll.is_authorized(&relay_message).await
        });
        assert!(
            is_authorized.is_ok(),
            "this implementation should never return Err"
        );
        let is_authorized = is_authorized.ok();
        assert_eq!(is_authorized, crate::allow().ok());
        assert_ne!(is_authorized, crate::deny().ok());
    }
}
