use crate::authenticator::{AuthorityMember, PreTrustedIdentities};
use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

/// This repository stores project members on the Authority node
#[async_trait]
pub trait AuthorityMembersRepository: Send + Sync + 'static {
    /// Return an existing member of the Project
    async fn get_member(&self, identifier: &Identifier) -> Result<Option<AuthorityMember>>;

    /// Return all members of the Project
    async fn get_members(&self) -> Result<Vec<AuthorityMember>>;

    /// Delete a member from the Project (unless it's pre-trusted)
    async fn delete_member(&self, identifier: &Identifier) -> Result<()>;

    /// Add a member to the Project
    async fn add_member(&self, member: AuthorityMember) -> Result<()>;

    /// Remove the old pre-trusted members and store new pre-trusted members
    async fn bootstrap_pre_trusted_members(
        &self,
        pre_trusted_identities: &PreTrustedIdentities,
    ) -> Result<()>;
}
