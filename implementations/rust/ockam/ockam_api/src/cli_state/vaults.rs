use std::fmt::{Display, Formatter};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ockam::identity::{Identities, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_node::database::SqlxDatabase;
use ockam_vault_aws::AwsSigningVault;

use crate::cli_state::{random_name, CliState, Result};
use crate::CliStateError;

/// The methods below support the creation and update of local vaults
///
///  - by default private keys are stored locally but they can also be stored in a KMS
///  - keys stored locally are stored with other application data in the local database if the default vault is used
///  - any additional vault stores its keys in a separate file
///
impl CliState {
    /// Create a vault with a given name
    /// If the path is not specified then:
    ///   - if this is the first vault then secrets are persisted in the main database
    ///   - if this is a new vault then secrets are persisted in $OCKAM_HOME/vault_name
    pub async fn create_named_vault(
        &self,
        vault_name: &Option<String>,
        path: &Option<PathBuf>,
    ) -> Result<NamedVault> {
        self.create_a_vault(vault_name, path, false).await
    }

    /// Create a KMS vault with a given name
    /// If the path is not specified then:
    ///   - if this is the first vault then secrets are persisted in the main database
    ///   - if this is a new vault then secrets are persisted in $OCKAM_HOME/vault_name
    pub async fn create_kms_vault(
        &self,
        vault_name: &Option<String>,
        path: &Option<PathBuf>,
    ) -> Result<NamedVault> {
        self.create_a_vault(vault_name, path, true).await
    }

    /// Delete an existing vault
    pub async fn delete_named_vault(&self, vault_name: &str) -> Result<()> {
        // first check that no identity is using the vault
        let identities_repository = self.identities_repository().await?;
        let identities_using_the_vault = identities_repository
            .get_named_identities_by_vault_name(vault_name)
            .await?;
        if !identities_using_the_vault.is_empty() {
            let identities_names = identities_using_the_vault
                .iter()
                .map(|i| i.name())
                .collect::<Vec<String>>();
            return Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Invalid,
                format!(
                    "the vault {vault_name} cannot be deleted. It is used by the following identities: {}",
                    identities_names.join(", ")
                ),
            )
            .into());
        };

        // now delete the vault and its file if there is a separate one
        let repository = self.vaults_repository().await?;
        let vault = repository.get_named_vault(vault_name).await?;
        if let Some(vault) = vault {
            repository.delete_named_vault(vault_name).await?;

            // if the vault is stored in a separate file
            // remove that file
            if vault.path != self.database_path() {
                let _ = std::fs::remove_file(vault.path);
            } else {
                // otherwise delete the tables used by the database vault
                self.purpose_keys_repository().await?.delete_all().await?;
                self.secrets_repository().await?.delete_all().await?;
            }
        }
        Ok(())
    }

    /// Delete all named identities
    pub async fn delete_all_named_identities(&self) -> Result<()> {
        let identities_repository = self.identities_repository().await?;
        let identities = identities_repository.get_named_identities().await?;
        for identity in identities {
            identities_repository
                .delete_identity(&identity.name())
                .await?;
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

    /// Return a vault if it already exists, otherwise
    /// Create a new vault using a default path: either the database path for the first vault
    /// or a path using the vault name
    pub async fn get_or_create_named_vault(&self, vault_name: &str) -> Result<NamedVault> {
        let vaults_repository = self.vaults_repository().await?;
        if let Ok(Some(existing_vault)) = vaults_repository.get_named_vault(vault_name).await {
            return Ok(existing_vault);
        }
        self.create_a_vault(&Some(vault_name.to_string()), &None, false)
            .await
    }

    /// Return the existing vault if there is only one
    /// If it doesn't exist, the vault is created with the name 'default'
    /// If there are more than one vaults, return an error
    pub async fn get_or_create_default_named_vault(&self) -> Result<NamedVault> {
        let vaults = self.vaults_repository().await?.get_named_vaults().await?;
        match &vaults[..] {
            [] => self.get_or_create_named_vault("default").await,
            [vault] => Ok(vault.clone()),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Invalid,
                format!(
                    "there are {} vaults, please specify which vault should be used",
                    vaults.len()
                ),
            )
            .into()),
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

    /// Move a vault file to another location if the vault is not the default vault
    /// contained in the main database
    pub async fn move_vault(&self, vault_name: &str, path: &Path) -> Result<()> {
        let repository = self.vaults_repository().await?;
        let vault = self.get_named_vault(vault_name).await?;
        if vault.path() == self.database_path() {
            return Err(ockam_core::Error::new(Origin::Api, Kind::Invalid, format!("The vault at path {:?} cannot be moved to {path:?} because this is the default vault", vault.path())).into());
        };

        // copy the file to the new location
        std::fs::copy(vault.path(), path)?;
        // update the path in the database
        repository.update_vault(vault_name, path).await?;
        // remove the old file
        std::fs::remove_file(vault.path())?;
        Ok(())
    }
}

/// Builder functions
impl CliState {
    /// Return an Identities struct using a specific Vault
    pub async fn make_identities(&self, vault: Vault) -> Result<Arc<Identities>> {
        Ok(Identities::builder()
            .await?
            .with_vault(vault)
            .with_change_history_repository(self.change_history_repository().await?)
            .with_identity_attributes_repository(self.identity_attributes_repository().await?)
            .with_purpose_keys_repository(self.purpose_keys_repository().await?)
            .build())
    }
}

/// Private functions
impl CliState {
    /// Create a vault with the given name and indicate if it is going to be used as a KMS vault
    /// If the vault with the same name already exists then an error is returned
    /// If there is already a file at the provided path, then an error is returned
    async fn create_a_vault(
        &self,
        vault_name: &Option<String>,
        path: &Option<PathBuf>,
        is_kms: bool,
    ) -> Result<NamedVault> {
        let vaults_repository = self.vaults_repository().await?;

        // determine the vault name to use if not given by the user
        let vault_name = match vault_name {
            Some(vault_name) => vault_name.clone(),
            None => self.make_vault_name().await?,
        };

        // verify that a vault with that name does not exist
        if vaults_repository
            .get_named_vault(&vault_name)
            .await?
            .is_some()
        {
            return Err(CliStateError::AlreadyExists {
                resource: "vault".to_string(),
                name: vault_name.to_string(),
            });
        }

        // determine the vault path
        // if the vault is the first vault we store the data directly in the main database
        // otherwise we open a new file with the vault name
        let path = match path {
            Some(path) => path.clone(),
            None => self.make_vault_path(&vault_name).await?,
        };

        // check if the new file can be created
        let path_taken = self.get_named_vault_with_path(&path).await?.is_some();
        if path_taken {
            return Err(CliStateError::AlreadyExists {
                resource: "vault path".to_string(),
                name: format!("{path:?}"),
            });
        } else {
            // create a new file if we need to store the vault data outside of the main database
            if path != self.database_path() {
                // similar to File::create_new which is unstable for now
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
            }
        };

        // store the vault metadata
        Ok(vaults_repository
            .store_vault(&vault_name, &path, is_kms)
            .await?)
    }

    /// Return the vault name to use for a vault:
    ///
    ///  - if a user has specified a name, use it
    ///  - otherwise if this is the first vault that is created, use the name 'default'
    ///  - finally create a random name
    ///
    async fn make_vault_name(&self) -> Result<String> {
        let vaults_repository = self.vaults_repository().await?;
        if vaults_repository.get_named_vaults().await?.is_empty() {
            Ok("default".to_string())
        } else {
            Ok(random_name())
        }
    }

    /// Decide which path to use for a vault path:
    ///   - if no vault has been using the main database, use it
    ///   - otherwise return a new path alongside the database $OCKAM_HOME/vault-{vault_name}
    ///
    async fn make_vault_path(&self, vault_name: &str) -> Result<PathBuf> {
        let vaults_repository = self.vaults_repository().await?;
        // is there already a vault using the main database?
        let is_database_path_available = vaults_repository
            .get_named_vaults()
            .await?
            .iter()
            .all(|v| v.path() != self.database_path());
        if is_database_path_available {
            Ok(self.database_path())
        } else {
            Ok(self.dir().join(format!("vault-{vault_name}")))
        }
    }

    async fn get_named_vault_with_path(&self, path: &Path) -> Result<Option<NamedVault>> {
        Ok(self
            .vaults_repository()
            .await?
            .get_named_vault_with_path(path)
            .await?)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct NamedVault {
    name: String,
    path: PathBuf,
    is_kms: bool,
}

impl NamedVault {
    /// Create a new named vault
    pub fn new(name: &str, path: PathBuf, is_kms: bool) -> Self {
        Self {
            name: name.to_string(),
            path,
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

    /// Return the vault path as a String
    pub fn path_as_string(&self) -> String {
        self.path.clone().to_string_lossy().to_string()
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
    use ockam::identity::models::{PurposeKeyAttestation, PurposeKeyAttestationSignature};
    use ockam::identity::Purpose;
    use ockam_vault::{
        ECDSASHA256CurveP256SecretKey, ECDSASHA256CurveP256Signature, HandleToSecret,
        SigningSecret, SigningSecretKeyHandle, X25519SecretKey, X25519SecretKeyHandle,
    };
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_create_named_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault
        let named_vault1 = cli.get_or_create_named_vault("vault1").await?;

        let result = cli.get_named_vault("vault1").await?;
        assert_eq!(result, named_vault1.clone());

        // another vault cannot be created with the same name
        let result = cli
            .create_named_vault(&Some("vault1".to_string()), &None)
            .await
            .ok();
        assert_eq!(result, None);

        // another vault cannot be created with the same path
        let result = cli
            .create_named_vault(&None, &Some(named_vault1.path()))
            .await
            .ok();
        assert_eq!(result, None);

        // the first created vault is the default one if it is the only one
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault1.clone());

        // the same vault is returned if queried for the second time
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault1.clone());

        // create another vault

        // it's not there initially
        let result = cli.get_named_vault("vault2").await.ok();
        assert_eq!(result, None);

        let named_vault2 = cli.get_or_create_named_vault("vault2").await?;

        let result = cli.get_named_vaults().await?;
        assert_eq!(result, vec![named_vault1.clone(), named_vault2.clone()]);

        // if there are more than 2 vaults then there is no default one
        let result = cli.get_or_create_default_named_vault().await.ok();
        assert_eq!(result, None);

        // a vault can be deleted
        cli.delete_named_vault("vault2").await?;
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, named_vault1);

        // all the vaults can be deleted
        cli.delete_all_named_vaults().await?;
        let result = cli.get_named_vaults().await?;
        assert!(result.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_get_default_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a default vault
        let vault = cli.get_or_create_default_named_vault().await?;
        assert_eq!(vault.name(), "default".to_string());

        // the same vault is returned the second time
        let result = cli.get_or_create_default_named_vault().await?;
        assert_eq!(result, vault);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_named_vault_or_default() -> Result<()> {
        let cli = CliState::test().await?;

        // create a default vault
        let vault1 = cli.get_or_create_default_named_vault().await?;
        assert_eq!(vault1.name(), "default".to_string());

        // that vault can be retrieved by passing its name
        let result = cli.get_named_vault_or_default(&Some(vault1.name())).await?;
        assert_eq!(result, vault1);

        // that vault can be retrieved by omitting its name since there is only one vault
        let result = cli.get_named_vault_or_default(&None).await?;
        assert_eq!(result, vault1);

        // if we create a second vault, it can be returned by name
        let vault2 = cli
            .create_named_vault(&Some("vault-2".to_string()), &None)
            .await?;
        let result = cli.get_named_vault_or_default(&Some(vault2.name())).await?;
        assert_eq!(result, vault2);

        // however if we don't specify a name, then we get an error because the user needs to be specific
        let result = cli.get_named_vault_or_default(&None).await.ok();
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_move_vault() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let cli_state_directory = db_file.path().parent().unwrap().join(random_name());
        let cli = CliState::create(cli_state_directory.clone()).await?;

        // create a vault
        let _ = cli.get_or_create_named_vault("vault1").await?;

        // try to move it. That should fail because the first vault is
        // stored in the main database
        let new_vault_path = cli_state_directory.join("new-vault-name");
        let result = cli.move_vault("vault1", &new_vault_path).await;
        assert!(result.is_err());

        // create a second vault
        let _ = cli.get_or_create_named_vault("vault2").await?;

        // try to move it. This should succeed
        let result = cli
            .move_vault("vault2", &cli_state_directory.join("new-vault-name"))
            .await;
        if let Err(e) = result {
            panic!("{}", e.to_string())
        };

        let vault = cli.get_named_vault("vault2").await?;
        assert_eq!(vault.path(), new_vault_path);
        assert!(vault.path().exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_vault_with_no_user_path() -> Result<()> {
        let cli = CliState::test().await?;

        // the first vault is stored in the main database with the name 'default'
        let result = cli.create_named_vault(&None, &None).await?;
        assert_eq!(result.name(), "default".to_string());
        assert_eq!(result.path(), cli.database_path());

        // the second vault is stored in a separate file, with a random name
        // that name is used to create the file name
        let result = cli.create_named_vault(&None, &None).await?;
        assert!(result
            .path_as_string()
            .ends_with(&format!("vault-{}", result.name())));

        // a third vault with a name is also stored in a separate file
        let result = cli
            .create_named_vault(&Some("secrets".to_string()), &None)
            .await?;
        assert_eq!(result.name(), "secrets".to_string());
        assert!(result.path_as_string().contains("vault-secrets"));

        // if we reset, we can check that the first vault gets the user defined name
        // instead of default
        cli.reset().await?;
        let cli = CliState::test().await?;
        let result = cli
            .create_named_vault(&Some("secrets".to_string()), &None)
            .await?;
        assert_eq!(result.name(), "secrets".to_string());
        assert_eq!(result.path(), cli.database_path());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_vault_with_a_user_path() -> Result<()> {
        let cli = CliState::test().await?;
        let vault_path = cli.database_path().parent().unwrap().join(random_name());

        let result = cli
            .create_named_vault(&Some("secrets".to_string()), &Some(vault_path.clone()))
            .await?;
        assert_eq!(result.name(), "secrets".to_string());
        assert_eq!(result.path(), vault_path);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault and populate the tables used by the vault
        let vault = cli.create_named_vault(&None, &None).await?;

        let purpose_keys_repository = cli.purpose_keys_repository().await?;
        let identity = cli.create_identity_with_name("name").await?;
        let purpose_key_attestation = PurposeKeyAttestation {
            data: vec![1, 2, 3],
            signature: PurposeKeyAttestationSignature::ECDSASHA256CurveP256(
                ECDSASHA256CurveP256Signature([1; 64]),
            ),
        };

        purpose_keys_repository
            .set_purpose_key(
                &identity.identifier(),
                Purpose::Credentials,
                &purpose_key_attestation,
            )
            .await?;

        let secrets_repository = cli.secrets_repository().await?;
        let handle1 =
            SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(vec![1, 2, 3]));
        let secret1 =
            SigningSecret::ECDSASHA256CurveP256(ECDSASHA256CurveP256SecretKey::new([1; 32]));
        secrets_repository
            .store_signing_secret(&handle1, secret1)
            .await?;

        let handle2 = X25519SecretKeyHandle(HandleToSecret::new(vec![1, 2, 3]));
        let secret2 = X25519SecretKey::new([1; 32]);
        secrets_repository
            .store_x25519_secret(&handle2, secret2)
            .await?;

        // the vault cannot be deleted if it still uses some identities
        let result = cli.delete_named_vault(&vault.name()).await;
        assert!(result.is_err());

        // when the vault is deleted, all the tables used by the vault are deleted too
        cli.delete_identity_by_name(&identity.name()).await?;
        cli.delete_named_vault(&vault.name()).await?;

        assert_eq!(
            purpose_keys_repository
                .get_purpose_key(&identity.identifier(), Purpose::Credentials)
                .await?,
            None
        );

        let result = secrets_repository.get_signing_secret(&handle1).await?;
        assert!(result.is_none());

        let result = secrets_repository.get_x25519_secret(&handle2).await?;
        assert!(result.is_none());

        Ok(())
    }
}
