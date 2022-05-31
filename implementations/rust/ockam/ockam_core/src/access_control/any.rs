use crate::access_control::AccessControl;
use crate::{async_trait, compat::boxed::Box, LocalMessage, Result};

/// Allows message that are allowed buy either AccessControls
pub struct AnyAccessControl<F: AccessControl, S: AccessControl> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: AccessControl, S: AccessControl> AnyAccessControl<F, S> {
    /// Constructor
    pub fn new(first: F, second: S) -> Self {
        AnyAccessControl { first, second }
    }
}

#[async_trait]
impl<F: AccessControl, S: AccessControl> AccessControl for AnyAccessControl<F, S> {
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        Ok(self.first.is_authorized(local_msg).await?
            || self.second.is_authorized(local_msg).await?)
    }
}

// TODO: Tests
