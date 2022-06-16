use crate::ExternalLocalInfo;
use ockam_core::access_control::AccessControl;
use ockam_core::{allow, LocalMessage, Result};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, vec::Vec},
    TransportType,
};

/// Allows only messages that originate from this node
#[derive(Debug)]
pub struct LocalOriginOnly;

#[async_trait]
impl AccessControl for LocalOriginOnly {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        Ok(ExternalLocalInfo::find_info(local_msg).is_err())
    }
}

/// Allows only messages coming from specified set of transport types. Also allows Local messages
#[derive(Debug)]
pub struct AllowedTransport {
    allowed_transport: Vec<TransportType>,
}

impl AllowedTransport {
    /// Constructor
    pub fn multiple(allowed_transport: Vec<TransportType>) -> Self {
        Self { allowed_transport }
    }

    /// Constructor
    pub fn single(allowed_transport: TransportType) -> Self {
        Self {
            allowed_transport: vec![allowed_transport],
        }
    }
}

#[async_trait]
impl AccessControl for AllowedTransport {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        for transport in ExternalLocalInfo::find_all(local_msg)?
            .into_iter()
            .map(|x| x.transport_type())
        {
            if !self.allowed_transport.contains(&transport) {
                return Ok(false);
            }
        }

        allow()
    }
}
