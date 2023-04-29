use crate::compat::boxed::Box;
use crate::{
    async_trait, IncomingAccessControl, OutgoingAccessControl, RelayMessage, Result, LOCAL,
};

/// Allows only messages to local workers
#[derive(Debug)]
pub struct LocalOnwardOnly;

#[async_trait]
impl OutgoingAccessControl for LocalOnwardOnly {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let next_hop = relay_msg.onward_route().next()?;

        // Check if next hop is local (note that further hops may be non-local)
        if next_hop.transport_type() != LOCAL {
            return crate::deny();
        }

        crate::allow()
    }
}

/// Allows only messages that originate from this node
#[derive(Debug)]
pub struct LocalSourceOnly;

#[async_trait]
impl IncomingAccessControl for LocalSourceOnly {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        if relay_msg.source().transport_type() != LOCAL {
            return crate::deny();
        }

        crate::allow()
    }
}

#[cfg(test)]
mod tests {
    use crate::compat::future::poll_once;
    use crate::{
        route, Address, IncomingAccessControl, LocalMessage, LocalOnwardOnly, LocalSourceOnly,
        OutgoingAccessControl, RelayMessage, Result, TransportMessage, TransportType,
    };

    #[test]
    fn test_onward() -> Result<()> {
        let local_onward_address = Address::random_local();
        let external_onward_address = Address::random(TransportType::new(1));
        let source_address = Address::random_local();

        let ac = LocalOnwardOnly;

        let msg = LocalMessage::new(
            TransportMessage::v1(local_onward_address.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), local_onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(external_onward_address.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), external_onward_address.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(local_onward_address.clone(), route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address.clone(), external_onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(external_onward_address, route![], vec![]),
            vec![],
        );
        let msg = RelayMessage::new(source_address, local_onward_address, msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }

    #[test]
    fn test_source() -> Result<()> {
        let local_source_address = Address::random_local();
        let external_source_address = Address::random(TransportType::new(1));
        let onward_address = Address::random_local();

        let ac = LocalSourceOnly;

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![local_source_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(local_source_address.clone(), onward_address.clone(), msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![external_source_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(external_source_address.clone(), onward_address.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![local_source_address.clone()],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(external_source_address.clone(), onward_address.clone(), msg);

        assert!(!poll_once(async { ac.is_authorized(&msg).await })?);

        let msg = LocalMessage::new(
            TransportMessage::v1(
                onward_address.clone(),
                route![external_source_address],
                vec![],
            ),
            vec![],
        );
        let msg = RelayMessage::new(local_source_address, onward_address, msg);

        assert!(poll_once(async { ac.is_authorized(&msg).await })?);

        Ok(())
    }
}
