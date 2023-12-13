use ockam::identity::models::ChangeHistory;
use ockam::identity::{Identifier, Identity};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_vault::{HandleToSecret, SigningSecretKeyHandle};

use crate::cli_state::{random_name, CliState, Result};

/// The methods below allow the creation named identities.
/// A NamedIdentity is an identity that is associated to a name in order to be more easily
/// retrieved when necessary.
///
/// A NamedIdentity can also be set as the default identity so that it is implicitly picked up
/// when running some commands:
///
///  - the first created identity is always the default one.
///  - when we need to use the default identity, it is created in case it did not exist before
///  - the name of the default identity, if it has been created implicitly is always "default"
///
/// In order to create an identity we need to have a Vault:
///
///  - if a vault has already been created, the vault name can be provided
///  - otherwise we used the default vault, which is created if it does not exist the first time we need it
///
impl CliState {
    /// Create an identity associated with a name and a specific vault name
    /// If there is already an identity with that name, return its identifier
    pub async fn create_identity_with_name_and_vault(
        &self,
        name: &str,
        vault_name: &str,
    ) -> Result<NamedIdentity> {
        if let Ok(identity) = self.get_named_identity(name).await {
            return Ok(identity);
        };

        let vault = self.get_named_vault(vault_name).await?;
        let identities = self.make_identities(vault.vault().await?).await?;
        let identity = identities.identities_creation().create_identity().await?;

        self.store_named_identity(&identity, name, &vault.name())
            .await
    }

    /// Create an identity associated with a name, using the default vault
    /// If there is already an identity with that name, return its identifier
    pub async fn create_identity_with_name(&self, name: &str) -> Result<NamedIdentity> {
        let vault = self.get_or_create_default_named_vault().await?;
        self.create_identity_with_name_and_vault(name, &vault.name())
            .await
    }

    /// Create an identity with specific key id.
    /// This method is used when the vault is a KMS vault and we just need to store a key id
    /// for the identity key existing in the KMS
    pub async fn create_identity_with_key_id(
        &self,
        name: &str,
        vault_name: &str,
        key_id: &str,
    ) -> Result<NamedIdentity> {
        let vault = self.get_named_vault(vault_name).await?;

        // Check that the vault is an KMS vault
        if !vault.is_kms() {
            return Err(Error::new(
                Origin::Api,
                Kind::Misuse,
                format!("Vault {vault_name} is not a KMS vault"),
            )
            .into());
        };

        let handle = SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(
            key_id.as_bytes().to_vec(),
        ));

        // create the identity
        let identities = self.make_identities(vault.vault().await?).await?;
        let identifier = identities
            .identities_creation()
            .identity_builder()
            .with_existing_key(handle)
            .build()
            .await?
            .clone();

        self.store_named_identity(&identifier, name, vault_name)
            .await
    }
}

/// The methods below allow to query identities:
///
///  - all of them
///  - one identity by name
///  - the identifier of a named identity
///  - etc...
///
/// Note that these methods return an Error when an identity is not found.
/// We assume, when using them that there is already an identity created with a given name.
///
impl CliState {
    /// Return all named identities
    pub async fn get_named_identities(&self) -> Result<Vec<NamedIdentity>> {
        Ok(self
            .identities_repository()
            .await?
            .get_named_identities()
            .await?)
    }

    /// Return a named identity given its name
    pub async fn get_named_identity(&self, name: &str) -> Result<NamedIdentity> {
        let repository = self.identities_repository().await?;
        match repository.get_named_identity(name).await? {
            Some(identity) => Ok(identity),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no identity found with name {}", name),
            )
            .into()),
        }
    }

    /// Return a named identity given its name or the default named identity
    pub async fn get_named_identity_or_default(
        &self,
        name: &Option<String>,
    ) -> Result<NamedIdentity> {
        match name {
            Some(name) => self.get_named_identity(name).await,
            None => self.get_or_create_default_named_identity().await,
        }
    }

    /// Return the identifier of a named identity
    pub async fn get_identifier_by_name(&self, name: &str) -> Result<Identifier> {
        Ok(self.get_named_identity(name).await?.identifier())
    }

    /// Return the identifier for identity given an optional name.
    /// If that name is None, then we return the identifier of the default identity
    pub async fn get_identifier_by_optional_name(
        &self,
        name: &Option<String>,
    ) -> Result<Identifier> {
        let repository = self.identities_repository().await?;
        let result = match name {
            Some(name) => repository.get_identifier(name).await?,
            None => repository
                .get_default_named_identity()
                .await?
                .map(|i| i.identifier()),
        };

        result.ok_or_else(|| Self::missing_identifier(name).into())
    }

    /// Return a full identity from its name
    /// Use the default identity if no name is given
    pub async fn get_identity_by_optional_name(&self, name: &Option<String>) -> Result<Identity> {
        let named_identity = match name {
            Some(name) => {
                self.identities_repository()
                    .await?
                    .get_named_identity(name)
                    .await?
            }

            None => {
                self.identities_repository()
                    .await?
                    .get_default_named_identity()
                    .await?
            }
        };
        match named_identity {
            Some(identity) => {
                let change_history = self.get_change_history(&identity.identifier()).await?;
                let identity_vault = self
                    .get_named_vault(&identity.vault_name)
                    .await?
                    .vault()
                    .await?;
                Ok(Identity::import_from_change_history(
                    Some(&identity.identifier()),
                    change_history,
                    identity_vault.verifying_vault,
                )
                .await?)
            }
            None => Err(Self::missing_identifier(name).into()),
        }
    }

    /// Return the identity with the given identifier
    pub async fn get_identity(&self, identifier: &Identifier) -> Result<Identity> {
        match self
            .change_history_repository()
            .await?
            .get_change_history(identifier)
            .await?
        {
            Some(change_history) => {
                Ok(Identity::create_from_change_history(&change_history).await?)
            }
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no identity found for identifier {identifier}"),
            )
            .into()),
        }
    }

    /// Return the name of the default identity.
    /// This function creates the default identity if it does not exist!
    pub async fn get_default_identity_name(&self) -> Result<String> {
        Ok(self.get_or_create_default_named_identity().await?.name())
    }

    /// Return the default named identity
    /// This function creates the default identity if it does not exist!
    pub async fn get_or_create_default_named_identity(&self) -> Result<NamedIdentity> {
        match self
            .identities_repository()
            .await?
            .get_default_named_identity()
            .await?
        {
            Some(named_identity) => Ok(named_identity),
            None => self.create_identity_with_name(&random_name()).await,
        }
    }

    /// Return:
    /// - the given name if defined
    /// - or the name of the default identity (which is created if it does not already exist!)
    pub async fn get_identity_name_or_default(&self, name: &Option<String>) -> Result<String> {
        match name {
            Some(name) => Ok(name.clone()),
            None => self.get_default_identity_name().await,
        }
    }

    /// Return the named identity with the given identifier
    pub async fn get_named_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<NamedIdentity> {
        match self
            .identities_repository()
            .await?
            .get_named_identity_by_identifier(identifier)
            .await?
        {
            Some(named_identity) => Ok(named_identity),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no named identity found for identifier {identifier}"),
            )
            .into()),
        }
    }

    /// Return true if there is an identity with that name and it is the default one
    pub async fn is_default_identity_by_name(&self, name: &str) -> Result<bool> {
        Ok(self
            .identities_repository()
            .await?
            .get_named_identity(name)
            .await?
            .map(|n| n.is_default())
            .unwrap_or(false))
    }
}

/// The following methods allow to update existing named identities
impl CliState {
    /// Set a named identity as the default
    /// Return an error if that identity does not exist
    pub async fn set_as_default_identity(&self, name: &str) -> Result<()> {
        Ok(self
            .identities_repository()
            .await?
            .set_as_default(name)
            .await?)
    }

    /// Delete an identity by name:
    ///
    ///  - check that the identity is not used by a node first
    ///  - then remove the the name association to the identity
    ///  - and remove the identity change history
    ///
    pub async fn delete_identity_by_name(&self, name: &str) -> Result<()> {
        let nodes = self.get_nodes_by_identity_name(name).await?;
        if nodes.is_empty() {
            if let Some(identifier) = self
                .identities_repository()
                .await?
                .delete_identity(name)
                .await?
            {
                self.change_history_repository()
                    .await?
                    .delete_change_history(&identifier)
                    .await?;
            };
            Ok(())
        } else {
            let node_names: Vec<String> = nodes.iter().map(|n| n.name()).collect();
            Err(Error::new(
                Origin::Api,
                Kind::Invalid,
                format!(
                    "The identity named {name} cannot be deleted because it is used by the node(s): {}",
                    node_names.join(", ")
                ),
            )
                .into())
        }
    }
}

/// Support methods
impl CliState {
    /// Once a identity has been created, store it.
    /// If there is no previous default identity we set it as the default identity
    async fn store_named_identity(
        &self,
        identifier: &Identifier,
        name: &str,
        vault_name: &str,
    ) -> Result<NamedIdentity> {
        let repository = self.identities_repository().await?;

        // If there is no previously created identity we set this identity as the default one
        let is_default_identity = repository.get_default_named_identity().await?.is_none();
        let mut named_identity = repository
            .store_named_identity(identifier, name, vault_name)
            .await?;
        if is_default_identity {
            repository
                .set_as_default_by_identifier(&named_identity.identifier())
                .await?;
            named_identity = named_identity.set_as_default();
        }
        Ok(named_identity)
    }

    /// Return the change history of a persisted identity
    async fn get_change_history(&self, identifier: &Identifier) -> Result<ChangeHistory> {
        match self
            .change_history_repository()
            .await?
            .get_change_history(identifier)
            .await?
        {
            Some(change_history) => Ok(change_history),
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("identity not found for identifier {}", identifier),
            )
            .into()),
        }
    }

    fn missing_identifier(name: &Option<String>) -> Error {
        let message = name
            .clone()
            .map_or("no default identifier found".to_string(), |n| {
                format!("no identifier found with name {}", n)
            });
        Error::new(Origin::Api, Kind::NotFound, message)
    }
}

/// A named identity associates a name with a persisted identity.
/// This is a convenience for users since they can refer to an identity by the name "alice"
/// instead of the identifier "I1234561234561234561234561234561234561234"
///
/// Additionally one identity can be marked as being the default identity and taken to
/// establish a secure channel or create credentials without having to specify it.
#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct NamedIdentity {
    identifier: Identifier,
    name: String,
    vault_name: String,
    is_default: bool,
}

impl NamedIdentity {
    /// Create a new named identity
    pub fn new(identifier: Identifier, name: String, vault_name: String, is_default: bool) -> Self {
        Self {
            identifier,
            name,
            vault_name,
            is_default,
        }
    }

    /// Return the identity identifier
    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    /// Return the identity name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the vault name
    pub fn vault_name(&self) -> String {
        self.vault_name.clone()
    }

    /// Return true if this identity is the default one
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Return a NamedIdentity where is_default is set to true
    pub fn set_as_default(&self) -> NamedIdentity {
        let mut result = self.clone();
        result.is_default = true;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_identity_with_a_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault first
        let vault_name = "vault-name";
        let _ = cli.create_named_vault(vault_name).await?;

        // then create an identity
        let identity_name = "identity-name";
        let identity = cli
            .create_identity_with_name_and_vault(identity_name, vault_name)
            .await?;
        let expected = cli.get_named_identity(identity_name).await?;
        assert_eq!(identity, expected);

        // don't recreate the identity if it already exists with that name
        let _ = cli
            .create_identity_with_name_and_vault(identity_name, vault_name)
            .await?;
        let identities = cli.get_named_identities().await?;
        assert_eq!(identities.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_identity_with_the_default_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create an identity using the default vault
        let identity_name = "identity-name";
        let identity = cli.create_identity_with_name(identity_name).await?;
        let expected = cli.get_named_identity(identity_name).await?;
        assert_eq!(identity, expected);

        // check that the identity uses the default vault
        let default_vault = cli.get_or_create_default_named_vault().await?;
        assert_eq!(identity.vault_name, default_vault.name());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_default_identity() -> Result<()> {
        let cli = CliState::test().await?;

        // when we retrieve the default identity, we create it if it doesn't exist
        let identity = cli.get_or_create_default_named_identity().await?;

        // when the identity is created there is a change history + a named identity
        let result = cli.get_change_history(&identity.identifier()).await;
        assert!(result.is_ok());

        let result = cli.get_named_identity(&identity.name()).await;
        assert!(result.is_ok());

        // check that the identity uses the default vault
        let default_vault = cli.get_or_create_default_named_vault().await?;
        assert_eq!(identity.vault_name, default_vault.name());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_identity() -> Result<()> {
        let cli = CliState::test().await?;
        let identity = cli.create_identity_with_name("name").await?;

        // when the identity is created there is a change history + a named identity
        let result = cli.get_change_history(&identity.identifier()).await;
        assert!(result.is_ok());

        let result = cli.get_named_identity(&identity.name()).await;
        assert!(result.is_ok());

        // now if we delete the identity there is no more persisted data
        cli.delete_identity_by_name(&identity.name()).await?;
        let result = cli.get_change_history(&identity.identifier()).await;
        assert!(result.is_err());

        let result = cli.get_named_identity(&identity.name()).await;
        assert!(result.is_err());

        Ok(())
    }
}
