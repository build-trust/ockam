use crate::access_control::{IncomingAccessControl, OutgoingAccessControl};
use crate::compat::sync::Arc;
use crate::compat::vec::Vec;
use crate::{async_trait, compat::boxed::Box, RelayMessage, Result};

/// Allows message that are allowed by all [`IncomingAccessControl`]s
#[derive(Debug)]
pub struct AllIncomingAccessControl(Vec<Arc<dyn IncomingAccessControl>>);

impl AllIncomingAccessControl {
    /// Constructor
    pub fn new(access_controls: Vec<Arc<dyn IncomingAccessControl>>) -> Self {
        Self(access_controls)
    }
}

#[async_trait]
impl IncomingAccessControl for AllIncomingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for ac in &self.0 {
            if !ac.is_authorized(relay_msg).await? {
                return crate::deny();
            }
        }

        crate::allow()
    }
}

/// Allows message that are allowed by all [`OutgoingAccessControl`]s
#[derive(Debug)]
pub struct AllOutgoingAccessControl(Vec<Arc<dyn OutgoingAccessControl>>);

impl AllOutgoingAccessControl {
    /// Constructor
    pub fn new(access_controls: Vec<Arc<dyn OutgoingAccessControl>>) -> Self {
        Self(access_controls)
    }
}

#[async_trait]
impl OutgoingAccessControl for AllOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for ac in &self.0 {
            if !ac.is_authorized(relay_msg).await? {
                return crate::deny();
            }
        }

        crate::allow()
    }
}

// TODO: Tests
