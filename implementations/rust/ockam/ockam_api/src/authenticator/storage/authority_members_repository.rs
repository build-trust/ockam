use crate::authenticator::{AuthorityMember, PreTrustedIdentities};
use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

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

#[async_trait]
impl<T: AuthorityMembersRepository> AuthorityMembersRepository for AutoRetry<T> {
    async fn get_member(&self, identifier: &Identifier) -> Result<Option<AuthorityMember>> {
        retry!(self.wrapped.get_member(identifier))
    }

    async fn get_members(&self) -> Result<Vec<AuthorityMember>> {
        retry!(self.wrapped.get_members())
    }

    async fn delete_member(&self, identifier: &Identifier) -> Result<()> {
        retry!(self.wrapped.delete_member(identifier))
    }

    async fn add_member(&self, member: AuthorityMember) -> Result<()> {
        retry!(self.wrapped.add_member(member.clone()))
    }

    async fn bootstrap_pre_trusted_members(
        &self,
        pre_trusted_identities: &PreTrustedIdentities,
    ) -> Result<()> {
        retry!(self
            .wrapped
            .bootstrap_pre_trusted_members(pre_trusted_identities))
    }
}
