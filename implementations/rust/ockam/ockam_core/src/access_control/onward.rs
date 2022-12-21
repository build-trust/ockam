use crate::compat::boxed::Box;
use crate::compat::vec::Vec;
use crate::{Address, OutgoingAccessControl, RelayMessage, Result};

/// An Access Control type that allows messages to the given onward address to go through
/// Note that onward and destination addresses are different in some cases
#[derive(Debug)]
pub struct AllowOnwardAddress(pub Address);

#[async_trait]
impl OutgoingAccessControl for AllowOnwardAddress {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        // Check if next hop is equal to expected value. Further hops are not checked
        if &self.0 != onward_route.next()? {
            return crate::deny();
        }

        crate::allow()
    }
}

/// An Access Control type that allows messages to the given onward address to go through
/// Note that onward and destination addresses are different in some cases
#[derive(Debug)]
pub struct AllowOnwardAddresses(pub Vec<Address>);

#[async_trait]
impl OutgoingAccessControl for AllowOnwardAddresses {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        // Check if next hop is equal to expected value. Further hops are not checked
        if !self.0.contains(onward_route.next()?) {
            return crate::deny();
        }

        crate::allow()
    }
}

#[cfg(test)]
mod tests {
    use crate::compat::future::poll_once;
    use crate::{
        route, Address, AllowOnwardAddress, AllowOnwardAddresses, LocalMessage,
        OutgoingAccessControl, RelayMessage, Result, TransportMessage,
    };

    #[test]
    fn test_1_address() -> Result<()> {
        let onward_address1 = Address::random_local();
        let onward_address2 = Address::random_local();
        let source_address = Address::random_local();

        let ac = AllowOnwardAddress(onward_address1.clone());

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address1.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address2.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address2.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address1.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address2.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address2.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }

    #[test]
    fn test_2_addresses() -> Result<()> {
        let onward_address1 = Address::random_local();
        let onward_address2 = Address::random_local();
        let onward_address3 = Address::random_local();
        let source_address = Address::random_local();

        let ac = AllowOnwardAddresses(vec![onward_address1.clone(), onward_address2.clone()]);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address1.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address2.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address2.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address3.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address3.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address3.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address1.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address3.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }
}
