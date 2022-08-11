use crate::access_control::AccessControl;
use crate::{async_trait, compat::boxed::Box, RelayMessage, Result};

use core::fmt::{self, Debug};

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
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        Ok(self.first.is_authorized(relay_msg).await?
            || self.second.is_authorized(relay_msg).await?)
    }
}

impl<F: AccessControl, S: AccessControl> Debug for AnyAccessControl<F, S> {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AllowAny({:?} | {:?})", self.first, self.second)
    }
}

// TODO: Tests
