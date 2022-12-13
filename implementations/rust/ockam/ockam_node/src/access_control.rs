use crate::ExternalLocalInfo;
use ockam_core::access_control::AccessControl;
use ockam_core::{allow, deny, RelayMessage, Result, LOCAL};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, vec::Vec},
    TransportType,
};

use core::fmt::{self, Debug};

/// Allows only messages that originate from this node
#[derive(Debug)]
pub struct LocalOriginOnly;

#[async_trait]
impl AccessControl for LocalOriginOnly {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        // FIXME: @ac check only previous hop?
        // Ok(ExternalLocalInfo::find_info(&relay_msg.local_msg).is_err())

        // Check if next hop is equal to expected value. Further hops are not checked
        if relay_msg.source().transport_type() != LOCAL {
            return deny();
        }

        allow()
    }
}

/// Allows only messages coming from specified set of transport types. Also allows Local messages
/// TODO allow specification of port as well
pub struct AllowFromTransport {
    allowed_transports: Vec<TransportType>,
}

impl AllowFromTransport {
    /// Constructor
    pub fn multiple(allowed_transport: Vec<TransportType>) -> Self {
        Self {
            allowed_transports: allowed_transport,
        }
    }

    /// Constructor
    pub fn single(allowed_transport: TransportType) -> Self {
        Self {
            allowed_transports: vec![allowed_transport],
        }
    }
}

#[async_trait]
impl AccessControl for AllowFromTransport {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for transport in ExternalLocalInfo::find_all(relay_msg.local_message())?
            .into_iter()
            .map(|x| x.transport_type())
        {
            if !self.allowed_transports.contains(&transport) {
                return Ok(false);
            }
        }

        allow()
    }
}

impl Debug for AllowFromTransport {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AllowFromTransport({:?})", self.allowed_transports)
    }
}

/// Allows sending message to external nodes using pre-defined transport types
pub struct AllowToTransport {
    allowed_transports: Vec<TransportType>,
}

impl AllowToTransport {
    /// Constructor
    pub fn multiple(allowed_transport: Vec<TransportType>) -> Self {
        let mut t = allowed_transport;
        t.push(LOCAL);
        Self {
            allowed_transports: t,
        }
    }

    /// Constructor
    pub fn single(allowed_transport: TransportType) -> Self {
        Self {
            allowed_transports: vec![LOCAL, allowed_transport],
        }
    }
}

#[async_trait]
impl AccessControl for AllowToTransport {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        for transport_type in onward_route.iter().map(|x| x.transport_type()) {
            if !self.allowed_transports.contains(&transport_type) {
                return deny();
            }
        }

        allow()
    }
}

impl Debug for AllowToTransport {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AllowToTransport({:?})", self.allowed_transports)
    }
}
