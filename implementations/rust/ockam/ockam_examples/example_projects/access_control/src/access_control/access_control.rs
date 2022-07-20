//! [`AccessControl`] implementation which uses [`AbacAuthorization`]
//! to resolve authorization requests.

use crate::access_control::AbacLocalInfo;

use ockam::abac::AbacAuthorization;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{AccessControl, LocalMessage, Result};

use core::fmt::{self, Debug, Formatter};

/// Allows only messages that pass attribute based access control.
pub struct AttributeBasedAccessControl<A> {
    backend: A,
}

impl<A> Debug for AttributeBasedAccessControl<A> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "AttributeBasedAccessControl")
    }
}

impl<A> AttributeBasedAccessControl<A> {
    /// Create a new `AttributeBasedAccessControl` with the given
    /// [`AbacAuthorization`] backend.
    pub fn new(backend: A) -> Self
    where
        A: AbacAuthorization,
    {
        Self { backend }
    }

    /// Return a reference to the [`AbacAuthorization`] implementation.
    pub fn backend(&self) -> &A {
        &self.backend
    }
}

#[async_trait]
impl<A> AccessControl for AttributeBasedAccessControl<A>
where
    A: AbacAuthorization,
{
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        // pull request triple out of LocalMessage's LocalInfo
        let local_info = AbacLocalInfo::find_info(local_msg)?;

        tracing::debug!(
            "AttributeBasedAccessControl::is_authorized() -> {:?}",
            local_info
        );

        self.backend
            .is_authorized(
                &local_info.subject,
                &local_info.resource,
                &local_info.action,
            )
            .await
    }
}
