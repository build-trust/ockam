use crate::access_control::{IncomingAccessControl, OutgoingAccessControl};
use crate::compat::sync::Arc;
use crate::compat::vec::Vec;
use crate::{async_trait, compat::boxed::Box, RelayMessage, Result};

/// Allows message that are allowed by any of [`IncomingAccessControl`]s
#[derive(Debug)]
pub struct AnyIncomingAccessControl(Vec<Arc<dyn IncomingAccessControl>>);

impl AnyIncomingAccessControl {
    /// Constructor
    pub fn new(access_controls: Vec<Arc<dyn IncomingAccessControl>>) -> Self {
        Self(access_controls)
    }
}

#[async_trait]
impl IncomingAccessControl for AnyIncomingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for ac in &self.0 {
            if ac.is_authorized(relay_msg).await? {
                return crate::allow();
            }
        }

        crate::deny()
    }
}

/// Allows message that are allowed by any of [`OutgoingAccessControl`]s
#[derive(Debug)]
pub struct AnyOutgoingAccessControl(Vec<Arc<dyn OutgoingAccessControl>>);

impl AnyOutgoingAccessControl {
    /// Constructor
    pub fn new(access_controls: Vec<Arc<dyn OutgoingAccessControl>>) -> Self {
        Self(access_controls)
    }
}

#[async_trait]
impl OutgoingAccessControl for AnyOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        for ac in &self.0 {
            if ac.is_authorized(relay_msg).await? {
                return crate::allow();
            }
        }

        crate::deny()
    }
}

// TODO: Tests
