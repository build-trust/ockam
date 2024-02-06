use crate::compat::boxed::Box;
use crate::compat::vec::Vec;
use crate::{async_trait, Address, OutgoingAccessControl, RelayMessage, Result};

/// An Access Control type that allows messages to the given onward address to go through
/// Note that onward and destination addresses are different in some cases
#[derive(Debug)]
pub struct AllowOnwardAddress(pub Address);

impl AllowOnwardAddress {
    /// Convenience constructor
    pub fn new(address: impl Into<Address>) -> Self {
        Self(address.into())
    }
}

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
    use crate::{
        route, Address, AllowOnwardAddress, AllowOnwardAddresses, LocalMessage,
        OutgoingAccessControl, RelayMessage, Result,
    };

    #[tokio::test]
    async fn test_1_address() -> Result<()> {
        let onward_address1 = Address::random_local();
        let onward_address2 = Address::random_local();
        let source_address = Address::random_local();

        let ac = AllowOnwardAddress(onward_address1.clone());

        let msg = LocalMessage::new().with_onward_route(route![onward_address1.clone()]);
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new()
            .with_onward_route(route![onward_address2.clone()])
            .with_return_route(route![]);
        let msg = RelayMessage::new(source_address.clone(), onward_address2.clone(), msg);

        assert!(!ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new()
            .with_onward_route(route![onward_address1.clone()])
            .with_return_route(route![]);
        let msg = RelayMessage::new(source_address.clone(), onward_address2.clone(), msg);

        assert!(ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new().with_onward_route(route![onward_address2]);
        let msg = RelayMessage::new(source_address, onward_address1, msg);

        assert!(!ac.is_authorized(&msg).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_2_addresses() -> Result<()> {
        let onward_address1 = Address::random_local();
        let onward_address2 = Address::random_local();
        let onward_address3 = Address::random_local();
        let source_address = Address::random_local();

        let ac = AllowOnwardAddresses(vec![onward_address1.clone(), onward_address2.clone()]);

        let msg = LocalMessage::new().with_onward_route(route![onward_address1.clone()]);
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new().with_onward_route(route![onward_address2.clone()]);
        let msg = RelayMessage::new(source_address.clone(), onward_address2, msg);

        assert!(ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new().with_onward_route(route![onward_address3.clone()]);
        let msg = RelayMessage::new(source_address.clone(), onward_address3.clone(), msg);

        assert!(!ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new().with_onward_route(route![onward_address3.clone()]);
        let msg = RelayMessage::new(source_address.clone(), onward_address1.clone(), msg);

        assert!(!ac.is_authorized(&msg).await?);

        let msg = LocalMessage::new().with_onward_route(route![onward_address1.clone()]);
        let msg = RelayMessage::new(source_address, onward_address3, msg);

        assert!(ac.is_authorized(&msg).await?);

        Ok(())
    }
}
