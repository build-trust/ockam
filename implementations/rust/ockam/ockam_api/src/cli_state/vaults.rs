use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

use ockam::identity::{Identifier, Identities, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use ockam_vault_aws::AwsSigningVault;

use crate::cli_state::{random_name, CliState, Result};

impl CliState {
    /// Create a vault with the given name if it was not created before
    /// If no name is given use a random name as the default vault name
    /// If the vault was not created before return Ok(vault)
    /// Otherwise return Err(name of the vault)
    pub async fn create_named_vault(
        &self,
        vault_name: &Option<String>,
    ) -> Result<std::result::Result<NamedVault, String>> {
        match vault_name {
            Some(vault_name) => {
                if self.get_named_vault(vault_name).await.is_ok() {
                    Ok(Err(vault_name.clone()))
                } else {
                    let vault = self.create_vault(vault_name).await?;
                    Ok(Ok(vault))
                }
            }
            None => match self.get_default_vault().await.ok() {
                Some(vault) => Ok(Err(vault.name())),
                None => Ok(Ok(self.create_vault(&random_name()).await?)),
            },
        }
    }

    pub async fn create_vault(&self, vault_name: &str) -> Result<NamedVault> {
        self.create_a_vault(vault_name, false).await
    }

    pub async fn create_kms_vault(&self, vault_name: &str) -> Result<NamedVault> {
        self.create_a_vault(vault_name, true).await
    }

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

        let vault = vaults_repository
            .store_vault(vault_name, path, is_kms)
            .await?;
        if is_default_vault {
            vaults_repository.set_as_default(vault_name).await?;
        }
        Ok(vault)
    }

    pub async fn is_default_vault(&self, vault_name: &str) -> Result<bool> {
        Ok(self
            .vaults_repository()
            .await?
            .get_named_vault(vault_name)
            .await?
            .map(|v| v.is_default())
            .unwrap_or(false))
    }

    pub async fn set_default_vault(&self, vault_name: &str) -> Result<()> {
        Ok(self
            .vaults_repository()
            .await?
            .set_as_default(vault_name)
            .await?)
    }

    pub async fn get_vault_names(&self) -> Result<Vec<String>> {
        let named_vaults = self.vaults_repository().await?.get_named_vaults().await?;
        Ok(named_vaults.iter().map(|v| v.name()).collect())
    }

    pub async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        Ok(self.vaults_repository().await?.get_named_vaults().await?)
    }

    /// Return either the default vault or a vault with the given name
    pub async fn get_vault_or_default(&self, vault_name: &Option<String>) -> Result<Vault> {
        let vault_name = self.get_vault_name_or_default(vault_name).await?;
        self.get_named_vault(&vault_name).await?.vault().await
    }

    /// Return either the default vault or a vault with the given name
    pub async fn get_named_vault_or_default(
        &self,
        vault_name: &Option<String>,
    ) -> Result<NamedVault> {
        let vault_name = self.get_vault_name_or_default(vault_name).await?;
        self.get_named_vault(&vault_name).await
    }

    /// Return the vault with the given name
    pub async fn get_vault_by_name(&self, vault_name: &str) -> Result<Vault> {
        self.get_named_vault(vault_name).await?.vault().await
    }

    pub async fn get_vault_name_or_default(&self, vault_name: &Option<String>) -> Result<String> {
        match vault_name {
            Some(name) => Ok(name.clone()),
            None => self.get_default_vault_name().await,
        }
    }

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

    pub async fn get_identifier_vault(&self, identifier: &Identifier) -> Result<NamedVault> {
        if let Some(vault_name) = self
            .identities_repository()
            .await?
            .get_named_identity_by_identifier(identifier)
            .await?
            .map(|n| n.vault_name())
        {
            self.get_named_vault(&vault_name).await
        } else {
            Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no vault found for identifier {identifier}"),
            )
            .into())
        }
    }

    pub(crate) async fn get_default_vault(&self) -> Result<NamedVault> {
        let result = self.vaults_repository().await?.get_default_vault().await?;
        match result {
            Some(vault) => Ok(vault),
            None => self.create_vault(&random_name()).await,
        }
    }

    pub async fn get_default_vault_name(&self) -> Result<String> {
        Ok(self.get_default_vault().await?.name())
    }

    /// Return the vault with the given name
    pub async fn delete_vault(&self, vault_name: &str) -> Result<()> {
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

    pub async fn get_identities_for_vault(&self, vault: NamedVault) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .await?
            .with_vault(vault.vault().await?)
            .with_change_history_repository(self.change_history_repository().await?)
            .with_identity_attributes_repository(self.identity_attributes_repository().await?)
            .with_purpose_keys_repository(self.purpose_keys_repository().await?)
            .build())
    }

    pub async fn get_identities_with_vault(&self, vault_name: &str) -> Result<Arc<Identities>> {
        let vault = self.get_named_vault(vault_name).await?;
        self.get_identities_for_vault(vault).await
    }

    pub async fn get_identities_with_optional_vault_name(
        &self,
        vault_name: &Option<String>,
    ) -> Result<Arc<Identities>> {
        let vault_name = self.get_vault_name_or_default(vault_name).await?;
        self.get_identities_with_vault(&vault_name).await
    }

    pub async fn get_identities(&self) -> Result<Arc<Identities>> {
        self.get_identities_with_optional_vault_name(&None).await
    }

    /// Delete all vaults and their files
    pub async fn delete_all_vaults(&self) -> Result<()> {
        let vaults = self.vaults_repository().await?.get_named_vaults().await?;
        for vault in vaults {
            self.delete_vault(&vault.name()).await?;
        }
        Ok(())
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
            Ok(Vault::create_with_persistent_storage_path(self.path.as_path()).await?)
        }
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
