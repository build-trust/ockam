//! `AccessControl` implementation for `Abac`.

use core::fmt::{self, Debug, Formatter};

use crate::abac::{Abac, AbacLocalInfo, Action, Attribute, Resource, Subject};
use ockam_core::async_trait;
use ockam_core::{allow, AccessControl, LocalMessage, Result};

/// Allows only messages that pass attribute based access control
pub struct AttributeBasedAccessControl<ABAC> {
    abac: ABAC,
}

impl<ABAC> Debug for AttributeBasedAccessControl<ABAC> {
    fn fmt<'a>(&'a self, f: &mut Formatter) -> fmt::Result {
        write!(f, "AttributeBasedAccessControl")
    }
}

impl<ABAC> AttributeBasedAccessControl<ABAC> {
    /// Create a new `AttributeBasedAccessControl`
    pub fn new(abac: ABAC) -> Self
    where
        ABAC: Abac + Debug + Send + Sync + 'static,
    {
        Self { abac }
    }

    /// Return a reference to the `Abac` implementation
    pub fn abac(&self) -> &ABAC {
        &self.abac
    }
}

#[async_trait]
impl<ABAC> AccessControl for AttributeBasedAccessControl<ABAC>
where
    ABAC: Abac + Debug + Send + Sync + 'static,
{
    async fn is_authorized(&self, local_msg: &LocalMessage) -> Result<bool> {
        // pull request triple out of LocalMessage's LocalInfo
        let local_info = AbacLocalInfo::find_info(local_msg)?;
        let parameters = local_info.parameters();

        tracing::debug!(
            "AttributeBasedAccessControl::is_authorized() -> {:?}",
            parameters
        );

        self.abac
            .is_authorized(
                &parameters.subject,
                &parameters.resource,
                &parameters.action,
            )
            .await
    }
}

// - will probably live in `ockam_api` ----------------------------------------

use crate::abac::Conditional;

/// Eventually wraps around the authenticated table service
#[derive(Debug)]
pub struct AuthenticatedTable;

#[async_trait]
impl Abac for AuthenticatedTable {
    async fn set_subject<I>(&self, _subject: Subject, _attrs: I) -> Result<()>
    where
        I: IntoIterator<Item = Attribute> + Send + 'static,
    {
        // TODO
        Ok(())
    }

    async fn del_subject(&self, _subject: &Subject) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn set_policy(
        &self,
        _resource: Resource,
        _action: Action,
        _condition: &Conditional,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn del_policy(&self, _resource: &Resource) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn is_authorized(
        &self,
        _subject: &Subject,
        _resource: &Resource,
        _action: &Action,
    ) -> Result<bool> {
        allow()
    }
}
