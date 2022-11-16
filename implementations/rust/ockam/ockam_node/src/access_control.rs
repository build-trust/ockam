use crate::ExternalLocalInfo;
use ockam_core::access_control::AccessControl;
use ockam_core::{allow, RelayMessage, Result};
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
        Ok(ExternalLocalInfo::find_info(&relay_msg.local_msg).is_err())
    }
}

/// Allows only messages coming from specified set of transport types. Also allows Local messages
/// TODO allow specification of port as well
pub struct AllowTransport {
    allowed_transports: Vec<TransportType>,
}

impl AllowTransport {
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
impl AccessControl for AllowTransport {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for transport in ExternalLocalInfo::find_all(&relay_msg.local_msg)?
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

impl Debug for AllowTransport {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AllowTransport({:?})", self.allowed_transports)
    }
}
