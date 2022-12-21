use crate::access_control::IncomingAccessControl;
use crate::{async_trait, compat::boxed::Box, RelayMessage, Result};

/// Allows message that are allowed buy both AccessControls
#[derive(Debug)]
pub struct AllAccessControl<F: IncomingAccessControl, S: IncomingAccessControl> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: IncomingAccessControl, S: IncomingAccessControl> AllAccessControl<F, S> {
    /// Constructor
    pub fn new(first: F, second: S) -> Self {
        AllAccessControl { first, second }
    }
}

#[async_trait]
impl<F: IncomingAccessControl, S: IncomingAccessControl> IncomingAccessControl
    for AllAccessControl<F, S>
{
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        Ok(self.first.is_authorized(relay_msg).await?
            && self.second.is_authorized(relay_msg).await?)
    }
}

// TODO: Tests
