use crate::cli_state::{random_name, CliState, CliStateError, Result};
use crate::colors::color_primary;
use crate::output::Output;
use crate::{fmt_log, fmt_ok, fmt_warn};
use colorful::Colorful;
use ockam::identity::{Identities, Vault};
use ockam_core::errcode::{Kind, Origin};
use ockam_node::database::SqlxDatabase;
use ockam_vault_aws::AwsSigningVault;
use std::fmt::Write;
use std::fmt::{Debug, Display, Formatter};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static DEFAULT_VAULT_NAME: &str = "default";

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
    #[instrument(skip_all, fields(vault_name = vault_name.clone()))]
    pub async fn create_named_vault(
        &self,
        vault_name: Option<String>,
        path: Option<PathBuf>,
        use_aws_kms: UseAwsKms,
    ) -> Result<NamedVault> {
        let vaults_repository = self.vaults_repository();

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

        // Determine if the vault needs to be created at a specific path
        // or if data can be stored in the main database directly
        match path {
            None => match self.vaults_repository().get_database_vault().await? {
                None => Ok(vaults_repository
                    .store_vault(&vault_name, VaultType::database(use_aws_kms))
                    .await?),
                Some(_) => {
                    let path = self.make_vault_path(&vault_name)?;
                    Ok(self
                        .create_local_vault(vault_name, &path, use_aws_kms)
                        .await?)
                }
            },
            Some(path) => Ok(self
                .create_local_vault(vault_name, &path, use_aws_kms)
                .await?),
        }
    }

    /// Delete an existing vault
    #[instrument(skip_all, fields(vault_name = vault_name))]
    pub async fn delete_named_vault(&self, vault_name: &str) -> Result<()> {
        // first check that no identity is using the vault
        let identities_repository = self.identities_repository();
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
            ))?;
        };

        // now delete the vault and its file if there is a separate one
        let repository = self.vaults_repository();
        let vault = repository.get_named_vault(vault_name).await?;
        if let Some(vault) = vault {
            repository.delete_named_vault(vault_name).await?;
            match vault.vault_type {
                VaultType::DatabaseVault { .. } => {
                    self.purpose_keys_repository().delete_all().await?;
                    self.secrets_repository().delete_all().await?;
                }
                VaultType::LocalFileVault { path, .. } => {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }

    /// Delete all named identities
    #[instrument(skip_all)]
    pub async fn delete_all_named_identities(&self) -> Result<()> {
        let identities_repository = self.identities_repository();
        let identities = identities_repository.get_named_identities().await?;
        for identity in identities {
            if let Err(err) = identities_repository
                .delete_identity(&identity.name())
                .await
            {
                self.notify_message(fmt_warn!(
                    "Failed to delete the identity {}: {err}",
                    color_primary(identity.name())
                ));
            }
        }
        Ok(())
    }

    /// Delete all vaults and their files
    #[instrument(skip_all)]
    pub async fn delete_all_named_vaults(&self) -> Result<()> {
        let vaults = self.vaults_repository().get_named_vaults().await?;
        for vault in vaults {
            if let Err(err) = self.delete_named_vault(&vault.name()).await {
                self.notify_message(fmt_warn!(
                    "Failed to delete the vault {}: {err}",
                    color_primary(vault.name())
                ));
            }
        }
        Ok(())
    }
}

/// The methods below provide an API to query named vaults.
impl CliState {
    /// Return all the named vaults
    #[instrument(skip_all)]
    pub async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        Ok(self.vaults_repository().get_named_vaults().await?)
    }

    /// Return the vault with a given name
    /// and raise an error if the vault is not found
    #[instrument(skip_all, fields(vault_name = vault_name))]
    pub async fn get_named_vault(&self, vault_name: &str) -> Result<NamedVault> {
        let result = self.vaults_repository().get_named_vault(vault_name).await?;
        Ok(result.ok_or_else(|| {
            ockam_core::Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("no vault found with name {vault_name}"),
            )
        })?)
    }

    /// Return a vault if it already exists, otherwise
    /// Create a new vault using a default path: either the database path for the first vault
    /// or a path using the vault name
    #[instrument(skip_all, fields(vault_name = vault_name))]
    pub async fn get_or_create_named_vault(&self, vault_name: &str) -> Result<NamedVault> {
        let vaults_repository = self.vaults_repository();

        if let Ok(Some(existing_vault)) = vaults_repository.get_named_vault(vault_name).await {
            return Ok(existing_vault);
        }

        self.notify_message(fmt_log!(
            "This Identity needs a Vault to store its secrets."
        ));
        let named_vault = if self
            .vaults_repository()
            .get_database_vault()
            .await?
            .is_none()
        {
            self.notify_message(fmt_log!(
                "There is no default Vault on this machine, creating one..."
            ));
            let vault = self
                .create_database_vault(vault_name.to_string(), UseAwsKms::No)
                .await?;
            self.notify_message(fmt_ok!(
                "Created a new Vault named {}.",
                color_primary(vault_name)
            ));
            vault
        } else {
            let vault = self
                .create_local_vault(
                    vault_name.to_string(),
                    &self.make_vault_path(vault_name)?,
                    UseAwsKms::No,
                )
                .await?;
            self.notify_message(fmt_ok!(
                "Created a new Vault named {} on your disk.",
                color_primary(vault_name)
            ));
            vault
        };

        if named_vault.is_default() {
            self.notify_message(fmt_ok!(
                "Marked this new Vault as your default Vault, on this machine.\n"
            ));
        }

        Ok(named_vault)
    }

    /// Return the existing vault if there is only one
    /// If it doesn't exist, the vault is created with the name 'default'
    /// If there are more than one vaults, return an error
    #[instrument(skip_all)]
    pub async fn get_or_create_default_named_vault(&self) -> Result<NamedVault> {
        let vaults = self.vaults_repository().get_named_vaults().await?;
        match &vaults[..] {
            [] => self.get_or_create_named_vault(DEFAULT_VAULT_NAME).await,
            [vault] => Ok(vault.clone()),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Invalid,
                format!(
                    "There are {} vaults, please specify which vault should be used",
                    vaults.len()
                ),
            ))?,
        }
    }

    /// Return either the default vault or a vault with the given name
    /// If the default vault is required and does not exist it is created.
    #[instrument(skip_all, fields(vault_name = vault_name.clone()))]
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
    #[instrument(skip_all, fields(vault_name = vault_name, path = path.to_string_lossy().to_string()))]
    pub async fn move_vault(&self, vault_name: &str, path: &Path) -> Result<()> {
        let repository = self.vaults_repository();
        let vault = self.get_named_vault(vault_name).await?;
        match vault.vault_type {
            VaultType::DatabaseVault { .. } => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Invalid,
                format!(
                    "The vault {} cannot be moved to {path:?} because this is the default vault",
                    vault.name()
                ),
            ))?,
            VaultType::LocalFileVault {
                path: old_path,
                use_aws_kms,
            } => {
                // copy the file to the new location
                std::fs::copy(&old_path, path)?;
                // update the path in the database
                repository
                    .update_vault(vault_name, VaultType::local_file(path, use_aws_kms))
                    .await?;
                // remove the old file
                std::fs::remove_file(old_path)?;
            }
        }
        Ok(())
    }

    /// Make a concrete vault based on the NamedVault metadata
    #[instrument(skip_all, fields(vault_name = named_vault.name))]
    pub async fn make_vault(&self, named_vault: NamedVault) -> Result<Vault> {
        let db = match named_vault.vault_type {
            VaultType::DatabaseVault { .. } => self.database(),
            VaultType::LocalFileVault { ref path, .. } =>
            // TODO: Avoid creating multiple dbs with the same file
            {
                SqlxDatabase::create_sqlite(path.as_path()).await?
            }
        };

        if named_vault.vault_type.use_aws_kms() {
            let mut vault = Vault::create_with_database(db);
            let aws_vault = Arc::new(AwsSigningVault::create().await?);
            vault.identity_vault = aws_vault.clone();
            vault.credential_vault = aws_vault;
            Ok(vault)
        } else {
            Ok(Vault::create_with_database(db))
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
    /// Create the database vault if it doesn't exist already
    async fn create_database_vault(
        &self,
        vault_name: String,
        use_aws_kms: UseAwsKms,
    ) -> Result<NamedVault> {
        match self.vaults_repository().get_database_vault().await? {
            None => Ok(self
                .vaults_repository()
                .store_vault(&vault_name, VaultType::database(use_aws_kms))
                .await?),
            Some(vault) => Err(CliStateError::AlreadyExists {
                resource: "database vault".to_string(),
                name: vault.name().to_string(),
            }),
        }
    }

    /// Create a vault store in a local file if the path has not been taken already
    async fn create_local_vault(
        &self,
        vault_name: String,
        path: &PathBuf,
        use_aws_kms: UseAwsKms,
    ) -> Result<NamedVault> {
        // check if the new file can be created
        let path_taken = self
            .get_named_vaults()
            .await?
            .iter()
            .any(|v| v.path() == Some(path.as_path()));
        if path_taken {
            Err(CliStateError::AlreadyExists {
                resource: "vault path".to_string(),
                name: format!("{path:?}"),
            })?;
        } else {
            // create a new file if we need to store the vault data outside of the main database
            // similar to File::create_new which is unstable for now
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(path)?;
        };
        Ok(self
            .vaults_repository()
            .store_vault(&vault_name, VaultType::local_file(path, use_aws_kms))
            .await?)
    }

    /// Return the vault name to use for a vault:
    ///
    ///  - if a user has specified a name, use it
    ///  - otherwise if this is the first vault that is created, use the name 'default'
    ///  - finally create a random name
    ///
    async fn make_vault_name(&self) -> Result<String> {
        let vaults_repository = self.vaults_repository();
        if vaults_repository.get_named_vaults().await?.is_empty() {
            Ok(DEFAULT_VAULT_NAME.to_string())
        } else {
            Ok(random_name())
        }
    }

    /// Decide which path to use for a vault path:
    ///   - otherwise return a new path alongside the database $OCKAM_HOME/vault-{vault_name}
    fn make_vault_path(&self, vault_name: &str) -> Result<PathBuf> {
        Ok(self.dir()?.join(format!("vault-{vault_name}")))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct NamedVault {
    name: String,
    vault_type: VaultType,
    is_default: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum VaultType {
    DatabaseVault {
        use_aws_kms: UseAwsKms,
    },
    LocalFileVault {
        path: PathBuf,
        use_aws_kms: UseAwsKms,
    },
}

impl Display for VaultType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Type: {}",
            match &self {
                VaultType::DatabaseVault { .. } => "INTERNAL",
                VaultType::LocalFileVault { .. } => "EXTERNAL",
            }
        )?;
        if self.use_aws_kms() {
            writeln!(f, "Uses AWS KMS: true",)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum UseAwsKms {
    Yes,
    No,
}

impl UseAwsKms {
    pub fn from(b: bool) -> Self {
        if b {
            UseAwsKms::Yes
        } else {
            UseAwsKms::No
        }
    }
}

impl VaultType {
    pub fn database(use_aws_kms: UseAwsKms) -> Self {
        VaultType::DatabaseVault { use_aws_kms }
    }

    pub fn local_file(path: impl Into<PathBuf>, use_aws_kms: UseAwsKms) -> Self {
        VaultType::LocalFileVault {
            path: path.into(),
            use_aws_kms,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            VaultType::DatabaseVault { .. } => None,
            VaultType::LocalFileVault { path, .. } => Some(path.as_path()),
        }
    }

    pub fn use_aws_kms(&self) -> bool {
        match self {
            VaultType::DatabaseVault { use_aws_kms } => use_aws_kms == &UseAwsKms::Yes,
            VaultType::LocalFileVault {
                path: _,
                use_aws_kms,
            } => use_aws_kms == &UseAwsKms::Yes,
        }
    }
}

impl NamedVault {
    /// Create a new named vault
    pub fn new(name: &str, vault_type: VaultType, is_default: bool) -> Self {
        Self {
            name: name.to_string(),
            vault_type,
            is_default,
        }
    }

    /// Return the vault name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the vault type
    pub fn vault_type(&self) -> VaultType {
        self.vault_type.clone()
    }

    /// Return true if this is the default vault
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    /// Return true if an AWS KMS is used to store signing keys
    pub fn use_aws_kms(&self) -> bool {
        self.vault_type.use_aws_kms()
    }

    /// Return the vault path if the vault data is stored in a local file
    pub fn path(&self) -> Option<&Path> {
        self.vault_type.path()
    }

    /// Return the vault path as a String
    pub fn path_as_string(&self) -> Option<String> {
        self.vault_type
            .path()
            .map(|p| p.to_string_lossy().to_string())
    }
}

impl Display for NamedVault {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "{}", self.vault_type)?;
        Ok(())
    }
}

impl Output for NamedVault {
    fn item(&self) -> crate::Result<String> {
        let mut output = String::new();
        writeln!(output, "Name: {}", self.name)?;
        writeln!(
            output,
            "Type: {}",
            match &self.vault_type {
                VaultType::DatabaseVault { .. } => "INTERNAL",
                VaultType::LocalFileVault { .. } => "EXTERNAL",
            }
        )?;
        if self.vault_type.use_aws_kms() {
            writeln!(output, "Uses AWS KMS: true",)?;
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam::identity::models::{PurposeKeyAttestation, PurposeKeyAttestationSignature};
    use ockam::identity::Purpose;
    use ockam_node::database::skip_if_postgres;
    use ockam_vault::{
        ECDSASHA256CurveP256SecretKey, ECDSASHA256CurveP256Signature, HandleToSecret,
        SigningSecret, SigningSecretKeyHandle, X25519SecretKey, X25519SecretKeyHandle,
    };

    #[tokio::test]
    async fn test_create_named_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault
        // since this is the first one, the data is stored in the database
        let named_vault1 = cli.get_or_create_named_vault("vault1").await?;

        let result = cli.get_named_vault("vault1").await?;
        assert_eq!(result, named_vault1.clone());

        // another vault cannot be created with the same name
        let result = cli
            .create_named_vault(Some("vault1".to_string()), None, UseAwsKms::No)
            .await
            .ok();
        assert_eq!(result, None);

        // the first created vault is the default one
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

        // that vault is using a local file
        assert!(named_vault2.path().is_some());
        // another vault cannot be created with the same path
        let result = cli
            .create_named_vault(
                Some("another name".to_string()),
                named_vault2.path().map(|p| p.to_path_buf()),
                UseAwsKms::No,
            )
            .await
            .ok();
        assert_eq!(result, None);

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
        assert_eq!(vault.name(), DEFAULT_VAULT_NAME.to_string());

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
        assert_eq!(vault1.name(), DEFAULT_VAULT_NAME.to_string());

        // that vault can be retrieved by passing its name
        let result = cli.get_named_vault_or_default(&Some(vault1.name())).await?;
        assert_eq!(result, vault1);

        // that vault can be retrieved by omitting its name since there is only one vault
        let result = cli.get_named_vault_or_default(&None).await?;
        assert_eq!(result, vault1);

        // if we create a second vault, it can be returned by name
        let vault2 = cli
            .create_named_vault(Some("vault-2".to_string()), None, UseAwsKms::No)
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
        let cli = CliState::test().await?;

        // create a vault
        let _ = cli.get_or_create_named_vault("vault1").await?;

        // try to move it. That should fail because the first vault is
        // stored in the main database
        let new_vault_path = cli.dir()?.join("new-vault-name");
        let result = cli.move_vault("vault1", &new_vault_path).await;
        assert!(result.is_err());

        // create a second vault
        let _ = cli.get_or_create_named_vault("vault2").await?;

        // try to move it. This should succeed
        let result = cli
            .move_vault("vault2", &cli.dir()?.join("new-vault-name"))
            .await;
        if let Err(e) = result {
            panic!("{}", e.to_string())
        };

        let vault = cli.get_named_vault("vault2").await?;
        assert_eq!(vault.path(), Some(new_vault_path.as_path()));
        assert!(new_vault_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_create_vault_with_no_user_path() -> Result<()> {
        let cli = CliState::test().await?;

        // the first vault is stored in the main database with the name 'default'
        let result = cli.create_named_vault(None, None, UseAwsKms::No).await?;
        assert_eq!(result.name(), DEFAULT_VAULT_NAME.to_string());
        assert_eq!(result.vault_type(), VaultType::database(UseAwsKms::No));

        // the second vault is stored in a separate file, with a random name
        // that name is used to create the file name
        let result = cli.create_named_vault(None, None, UseAwsKms::No).await?;
        assert!(result.path().is_some());
        assert!(result
            .path_as_string()
            .unwrap()
            .ends_with(&format!("vault-{}", result.name())));

        // a third vault with a name is also stored in a separate file
        let result = cli
            .create_named_vault(Some("secrets".to_string()), None, UseAwsKms::No)
            .await?;
        assert_eq!(result.name(), "secrets".to_string());
        assert!(result.path().is_some());
        assert!(result.path_as_string().unwrap().contains("vault-secrets"));

        // if we reset, we can check that the first vault gets the user defined name
        // instead of default.
        // We only test this for sqlite since we can't reset with postgres.

        skip_if_postgres(move || {
            let cli_clone = cli.clone();
            async move {
                cli_clone.reset().await?;
                let cli = CliState::test().await?;
                let result = cli
                    .create_named_vault(Some("secrets".to_string()), None, UseAwsKms::No)
                    .await?;
                assert_eq!(result.name(), "secrets".to_string());
                assert_eq!(result.vault_type(), VaultType::database(UseAwsKms::No));
                let result: Result<()> = Ok(());
                result
            }
        })
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_create_vault_with_a_user_path() -> Result<()> {
        let cli = CliState::test().await?;
        let vault_path = cli.dir()?.join(random_name());

        let result = cli
            .create_named_vault(
                Some("secrets".to_string()),
                Some(vault_path.clone()),
                UseAwsKms::No,
            )
            .await?;
        assert_eq!(result.name(), "secrets".to_string());
        assert_eq!(result.path(), Some(vault_path.as_path()));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_vault() -> Result<()> {
        let cli = CliState::test().await?;

        // create a vault and populate the tables used by the vault
        let vault = cli.create_named_vault(None, None, UseAwsKms::No).await?;

        let purpose_keys_repository = cli.purpose_keys_repository();
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

        let secrets_repository = cli.secrets_repository();
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
