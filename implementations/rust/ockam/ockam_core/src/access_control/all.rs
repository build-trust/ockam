use crate::access_control::AccessControl;
use crate::{async_trait, compat::boxed::Box, LocalMessage, Result};

/// Allows message that are allowed buy both AccessControls
#[derive(Debug)]
pub struct AllAccessControl<F: AccessControl, S: AccessControl> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: AccessControl, S: AccessControl> AllAccessControl<F, S> {
    /// Constructor
    pub fn new(first: F, second: S) -> Self {
        AllAccessControl { first, second }
    }
}

#[async_trait]
impl<F: AccessControl, S: AccessControl> AccessControl for AllAccessControl<F, S> {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        Ok(self.first.is_authorized(local_msg).await?
            && self.second.is_authorized(local_msg).await?)
    }
}

// TODO: Tests
