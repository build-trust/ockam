use crate::compat::boxed::Box;
use crate::compat::vec::Vec;
use crate::{Address, IncomingAccessControl, RelayMessage, Result};

/// An Access Control type that allows messages from the given source address to go through
/// Note that it's based on source address, not a first hop of return_route, which may be different
/// in some scenarios
#[derive(Debug)]
pub struct AllowSourceAddress(pub Address);

#[async_trait]
impl IncomingAccessControl for AllowSourceAddress {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if &self.0 == relay_msg.source() {
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

/// An Access Control type that allows messages from the given source addresses to go through
/// Note that it's based on source address, not a first hop of return_route, which may be different
/// in some scenarios
#[derive(Debug)]
pub struct AllowSourceAddresses(pub Vec<Address>);

#[async_trait]
impl IncomingAccessControl for AllowSourceAddresses {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if self.0.contains(relay_msg.source()) {
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compat::future::poll_once;
    use crate::{
        route, Address, AllowSourceAddress, AllowSourceAddresses, IncomingAccessControl,
        LocalMessage, RelayMessage, Result, TransportMessage,
    };

    #[test]
    fn test_1_address() -> Result<()> {
        // Testing situation when source and return addresses are different
        let source_address = Address::random_local();
        let return_address = Address::random_local();
        let onward_address = Address::random_local();

        let ac = AllowSourceAddress(source_address.clone());

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![return_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address.clone(), route![source_address], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(return_address, onward_address, msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }

    #[test]
    fn test_2_addresses() -> Result<()> {
        // Testing situation when source and return addresses are different
        let source_address1 = Address::random_local();
        let source_address2 = Address::random_local();
        let return_address = Address::random_local();
        let onward_address = Address::random_local();

        let ac = AllowSourceAddresses(vec![source_address1.clone(), source_address2.clone()]);

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![return_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(source_address1.clone(), onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![return_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(source_address2, onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(onward_address.clone(), route![source_address1], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(return_address, onward_address, msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }
}
