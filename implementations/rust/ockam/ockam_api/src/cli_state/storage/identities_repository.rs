use crate::cli_state::NamedIdentity;
use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

/// The identities repository stores metadata about identities
/// which change history have been stored in the ChangeHistoryRepository.
///
/// It allows to:
///
///  - associate a user name to an identity
///  - set one (and one only) identity as the default identity
///  - associate a vault name to an identity so that we know where the identity private keys can be found
///
/// By default the get/delete functions use the identity name as a parameter.
/// When they use the identity identifier instead, this is indicated in the function name:
/// e.g. get_named_identity_by_identifier()
///
#[async_trait]
pub trait IdentitiesRepository: Send + Sync + 'static {
    /// Associate a name to an identity
    async fn store_named_identity(
        &self,
        identifier: &Identifier,
        name: &str,
        vault_name: &str,
    ) -> Result<NamedIdentity>;

    /// Delete an identity given its name and return its identifier
    async fn delete_identity(&self, name: &str) -> Result<Option<Identifier>>;

    /// Delete an identity given its identifier and return its name
    async fn delete_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>>;

    /// Return the identifier associated to a named identity
    async fn get_identifier(&self, name: &str) -> Result<Option<Identifier>>;

    /// Return the name associated to an identifier
    async fn get_identity_name_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>>;

    /// Return the named identity with a specific name
    async fn get_named_identity(&self, name: &str) -> Result<Option<NamedIdentity>>;

    /// Return the named identity associated to an identifier
    async fn get_named_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<NamedIdentity>>;

    /// Return identities which have been given a name
    async fn get_named_identities(&self) -> Result<Vec<NamedIdentity>>;

    /// Return identities which have been given a name, and are using a specific vault
    async fn get_named_identities_by_vault_name(
        &self,
        vault_name: &str,
    ) -> Result<Vec<NamedIdentity>>;

    /// Set an identity as the default one, given its name
    async fn set_as_default(&self, name: &str) -> Result<()>;

    /// Set an identity as the default one, given its identifier
    async fn set_as_default_by_identifier(&self, identifier: &Identifier) -> Result<()>;

    /// Return the default named identity
    async fn get_default_named_identity(&self) -> Result<Option<NamedIdentity>>;
}

#[async_trait]
impl<T: IdentitiesRepository + Send + Sync + 'static> IdentitiesRepository for AutoRetry<T> {
    async fn store_named_identity(
        &self,
        identifier: &Identifier,
        name: &str,
        vault_name: &str,
    ) -> Result<NamedIdentity> {
        retry!(self
            .wrapped
            .store_named_identity(identifier, name, vault_name))
    }

    async fn delete_identity(&self, name: &str) -> Result<Option<Identifier>> {
        retry!(self.wrapped.delete_identity(name))
    }

    async fn delete_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>> {
        retry!(self.wrapped.delete_identity_by_identifier(identifier))
    }

    async fn get_identifier(&self, name: &str) -> Result<Option<Identifier>> {
        retry!(self.wrapped.get_identifier(name))
    }

    async fn get_identity_name_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>> {
        retry!(self.wrapped.get_identity_name_by_identifier(identifier))
    }

    async fn get_named_identity(&self, name: &str) -> Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_named_identity(name))
    }

    async fn get_named_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_named_identity_by_identifier(identifier))
    }

    async fn get_named_identities(&self) -> Result<Vec<NamedIdentity>> {
        retry!(self.wrapped.get_named_identities())
    }

    async fn get_named_identities_by_vault_name(
        &self,
        vault_name: &str,
    ) -> Result<Vec<NamedIdentity>> {
        retry!(self.wrapped.get_named_identities_by_vault_name(vault_name))
    }

    async fn set_as_default(&self, name: &str) -> Result<()> {
        retry!(self.wrapped.set_as_default(name))
    }

    async fn set_as_default_by_identifier(&self, identifier: &Identifier) -> Result<()> {
        retry!(self.wrapped.set_as_default_by_identifier(identifier))
    }

    async fn get_default_named_identity(&self) -> Result<Option<NamedIdentity>> {
        retry!(self.wrapped.get_default_named_identity())
    }
}
