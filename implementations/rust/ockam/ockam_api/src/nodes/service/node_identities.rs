use ockam::compat::sync::Arc;
use ockam::identity::{Identifier, Identities, Vault};
use ockam::Result;

use crate::cli_state::traits::StateDirTrait;
use crate::cli_state::CliState;

/// This struct supports identities operation that are either backed by
/// a specific vault or which are using the default vault
pub struct NodeIdentities {
    identities: Arc<Identities>,
    cli_state: CliState,
}

impl NodeIdentities {
    pub fn new(identities: Arc<Identities>, cli_state: CliState) -> NodeIdentities {
        NodeIdentities {
            identities,
            cli_state,
        }
    }

    pub(super) fn identities_vault(&self) -> Vault {
        self.identities.vault()
    }

    pub(crate) async fn get_identifier(&self, identity_name: String) -> Result<Identifier> {
        let identity_state = self.cli_state.identities.get(identity_name.as_str())?;
        Ok(identity_state.identifier())
    }

    /// Return an identities service, possibly backed by a specific vault
    pub(crate) async fn get_identities(
        &self,
        vault_name: Option<String>,
    ) -> Result<Arc<Identities>> {
        let vault = self.get_identities_vault(vault_name).await?;
        let repository = self.cli_state.identities.identities_repository().await?;
        Ok(Identities::builder()
            .with_vault(vault)
            .with_identities_repository(repository)
            .build())
    }

    /// Return either the default vault or a specific one
    pub(crate) async fn get_identities_vault(&self, vault_name: Option<String>) -> Result<Vault> {
        if let Some(vault) = vault_name {
            let existing_vault = self.cli_state.vaults.get(vault.as_str())?.get().await?;
            Ok(existing_vault)
        } else {
            Ok(self.identities_vault())
        }
    }
}
