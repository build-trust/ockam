use crate::cli_state::CliState;
use ockam::compat::sync::Arc;
use ockam::identity::{identities, Identities, IdentitiesCreation, IdentitiesKeys};
use ockam::identity::{IdentitiesVault, Identity};
use ockam::Result;

/// This struct supports identities operation that are either backed by
/// a specific vault or which are using the default vault
pub struct NodeIdentities {
    vault: Arc<dyn IdentitiesVault>,
    cli_state: CliState,
}

impl NodeIdentities {
    pub fn new(vault: Arc<dyn IdentitiesVault>, cli_state: CliState) -> NodeIdentities {
        NodeIdentities { vault, cli_state }
    }

    pub(super) fn identities_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.vault.clone()
    }

    /// Return an identity if it has been created with that name before
    /// If a vault name is specified, use it to validate the identity against that vault before returning it
    pub(crate) async fn get_identity(
        &self,
        identity_name: String,
        vault_name: Option<String>,
    ) -> Result<Option<Identity>> {
        let vault = self.get_identities_vault(vault_name).await?;
        if let Ok(idt_state) = self.cli_state.identities.get(identity_name.as_str()) {
            Ok(Some(idt_state.get(vault).await?))
        } else {
            Ok(None)
        }
    }

    /// Return an identities creation service backed up by the default vault
    pub(crate) async fn get_default_identities_creation(&self) -> Result<Arc<IdentitiesCreation>> {
        Ok(Arc::new(self.get_identities_creation(None).await?))
    }

    /// Return an identities keys service backed up by the default vault
    pub(crate) async fn get_default_identities_keys(&self) -> Result<Arc<IdentitiesKeys>> {
        Ok(identities::builder()
            .with_identities_vault(self.vault.clone())
            .build()
            .identities_keys())
    }

    /// Return an identities service for a specific identity
    pub(crate) async fn get_identities(
        &self,
        vault_name: Option<String>,
        identity_name: String,
    ) -> Result<Arc<Identities>> {
        let vault = self.get_identities_vault(vault_name).await?;
        let idt_state = self.cli_state.identities.get(identity_name.as_str())?;
        Ok(idt_state.make_identities(vault.clone()).await?)
    }

    /// Return an identities creations service
    pub(crate) async fn get_identities_creation(
        &self,
        vault_name: Option<String>,
    ) -> Result<IdentitiesCreation> {
        let vault = self.get_identities_vault(vault_name).await?;
        Ok(IdentitiesCreation::new(vault))
    }

    /// Return either the default vault or a specific one
    pub(crate) async fn get_identities_vault(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<dyn IdentitiesVault>> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(Arc::new(existing_vault))
        } else {
            Ok(self.identities_vault())
        }
    }

    /// Return a service to perform key operations
    pub(crate) async fn get_identities_keys(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<IdentitiesKeys>> {
        Ok(Arc::new(IdentitiesKeys::new(
            self.get_identities_vault(vault_name).await?,
        )))
    }
}
