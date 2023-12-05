use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use ockam::identity::{Identities, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_node::database::SqlxDatabase;
use ockam_vault_aws::AwsSigningVault;

use crate::cli_state::{random_name, CliState, Result};

/// The methods below support the creation and update of local vaults
///
///  - by default private keys are stored locally but they can also be stored in a KMS
///  - keys stored locally are stored with other application data in the local database if the default vault is used
///  - any additional vault stores its keys in a separate file
///
impl CliState {
    /// Create a vault with a given name
    /// The secrets persisted with this vault are stored under $OCKAM_HOME/vault_name
    pub async fn create_named_vault(&self, vault_name: &str) -> Result<NamedVault> {
        self.create_a_vault(vault_name, false).await
    }

    /// Create a KMS vault with a given name.
    /// A KMS vault only stores identifiers to secrets physically stored in a KMS like
    /// an AWS KMS (the only supported KMS implementation at the moment).
    ///
    /// The secrets persisted with this vault are stored under $OCKAM_HOME/vault_name
    pub async fn create_kms_vault(&self, vault_name: &str) -> Result<NamedVault> {
        self.create_a_vault(vault_name, true).await
    }

    /// Select a different vault to be the default vault
    pub async fn set_default_vault(&self, vault_name: &str) -> Result<()> {
        Ok(self
            .vaults_repository()
            .await?
            .set_as_default(vault_name)
            .await?)
    }

    /// Delete an existing vault
    pub async fn delete_named_vault(&self, vault_name: &str) -> Result<()> {
        let repository = self.vaults_repository().await?;
        let vault = repository.get_named_vault(vault_name).await?;
        if let Some(vault) = vault {
            repository.delete_vault(vault_name).await?;

            // if the vault is stored in a separate file
            // remove that file
            if vault.path != self.database_path() {
                let _ = std::fs::remove_file(vault.path);
            }
        }
        Ok(())
    }

    /// Delete all vaults and their files
    pub async fn delete_all_named_vaults(&self) -> Result<()> {
        let vaults = self.vaults_repository().await?.get_named_vaults().await?;
        for vault in vaults {
            self.delete_named_vault(&vault.name()).await?;
        }
        Ok(())
    }
}

/// The methods below provide an API to query named vaults.
impl CliState {
    /// Return all the named vaults
    pub async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        Ok(self.vaults_repository().await?.get_named_vaults().await?)
    }

    /// Return the vault with a given name
    /// and raise an error if the vault is not found
    pub async fn get_named_vault(&self, vault_name: &str) -> Result<NamedVault> {
        let result = self
            .vaults_repository()
            .await?
            .get_named_vault(vault_name)
            .await?;
        result.ok_or_else(|| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no vault found with name {vault_name}"),
            )
            .into()
        })
    }

    /// Return the default vault
    /// If it doesn't exist, the vault is created with a random name
    pub async fn get_or_create_default_named_vault(&self) -> Result<NamedVault> {
        let result = self.vaults_repository().await?.get_default_vault().await?;
        match result {
            Some(vault) => Ok(vault),
            None => self.create_named_vault(&random_name()).await,
        }
    }

    /// Return either the default vault or a vault with the given name
    /// If the default vault is required and does not exist it is created.
    pub async fn get_named_vault_or_default(
        &self,
        vault_name: &Option<String>,
    ) -> Result<NamedVault> {
        match vault_name {
            Some(name) => self.get_named_vault(name).await,
            None => self.get_or_create_default_named_vault().await,
        }
    }
}

/// Builder functions
impl CliState {
    /// Return an Identities struct using a specific Vault
    pub async fn make_identities(&self, vault: Vault) -> Result<Arc<Identities>> {
        Ok(Identities::create(self.database())
            .with_vault(vault)
            .build())
    }
}

/// Private functions
impl CliState {
    /// Create a vault with the given name and indicate if it is going to be used as a KMS vault
    /// The vault path is either
    /// - the database path if this is the first created vault (it is set as the default vault)
    /// - a file next to the database file, named 'vault_name'
    async fn create_a_vault(&self, vault_name: &str, is_kms: bool) -> Result<NamedVault> {
        let vaults_repository = self.vaults_repository().await?;

        // the first created vault is the default one
        let is_default_vault = vaults_repository.get_default_vault().await?.is_none();

        // if the vault is the default vault we store the data directly in the main database
        // otherwise we open a new file with the vault name
        let path = if is_default_vault {
            self.database_path()
        } else {
            self.dir().join(vault_name)
        };

        let mut vault = vaults_repository
            .store_vault(vault_name, path, is_kms)
            .await?;
        if is_default_vault {
            vaults_repository.set_as_default(vault_name).await?;
            vault = vault.set_as_default();
        }
        Ok(vault)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct NamedVault {
    name: String,
    path: PathBuf,
    is_default: bool,
    is_kms: bool,
}

impl NamedVault {
    /// Create a new named vault
    pub fn new(name: &str, path: PathBuf, is_default: bool, is_kms: bool) -> Self {
        Self {
            name: name.to_string(),
            path,
            is_default,
            is_kms,
        }
    }

    /// Return the vault name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the vault path
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Return true if this vault is the default one
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Return a copy of this vault as vault with the is_default flag set to true
    pub fn set_as_default(&self) -> NamedVault {
        let mut result = self.clone();
        result.is_default = true;
        result
    }

    /// Return true if this vault is a KMS vault
    pub fn is_kms(&self) -> bool {
        self.is_kms
    }

    pub async fn vault(&self) -> Result<Vault> {
        if self.is_kms {
            let mut vault = Vault::create().await?;
            let aws_vault = Arc::new(AwsSigningVault::create().await?);
            vault.identity_vault = aws_vault.clone();
            vault.credential_vault = aws_vault;
            Ok(vault)
        } else {
            Ok(Vault::create_with_database(self.database().await?))
        }
    }

    async fn database(&self) -> Result<Arc<SqlxDatabase>> {
        Ok(Arc::new(SqlxDatabase::create(self.path.as_path()).await?))
    }
}

impl Display for NamedVault {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(
            f,
            "Type: {}",
            match self.is_kms {
                true => "AWS KMS",
                false => "OCKAM",
            }
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_named_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault
        let named_vault1 = cli.create_named_vault("vault1").await?;

        let result = cli.get_named_vault("vault1").await?;
        assert_eq!(result, named_vault1.clone());

        // create another vault
        let named_vault2 = cli.create_named_vault("vault2").await?;

        let result = cli.get_named_vaults().await?;
        assert_eq!(result, vec![named_vault1.clone(), named_vault2.clone()]);

        // the first created vault is the default one
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault1.clone());

        // the default vault can be changed
        cli.set_default_vault("vault2").await?;
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault2.set_as_default());

        // a vault can be deleted
        cli.delete_named_vault("vault2").await?;
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault1.set_as_default());

        // all the vaults can be deleted
        cli.delete_all_named_vaults().await?;
        let result = cli.get_named_vaults().await?;
        assert!(result.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_default_named_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // the default vault is always available
        let vault = cli.get_or_create_default_named_vault().await?;
        assert!(vault.is_default());
        assert!(vault.path().starts_with(cli.dir()));

        Ok(())
    }
}
