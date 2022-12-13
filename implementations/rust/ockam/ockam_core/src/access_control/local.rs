use crate::compat::boxed::Box;
use crate::{AccessControl, RelayMessage, Result, LOCAL};

/// Allows only messages to local workers
#[derive(Debug)]
pub struct LocalOnwardOnly;

#[async_trait]
impl AccessControl for LocalOnwardOnly {
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
pub struct LocalOriginOnly;

#[async_trait]
impl AccessControl for LocalOriginOnly {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        // Check if next hop is equal to expected value. Further hops are not checked
        // FIXME: @ac
        if relay_msg.source().transport_type() != LOCAL {
            return crate::deny();
        }

        crate::allow()
    }
}
