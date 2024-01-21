use std::sync::Arc;

use ockam::identity::{Identities, IdentityAttributesRepository, SecureChannels};

use crate::bootstrapped_identities_store::{
    BootstrapedIdentityAttributesStore, PreTrustedIdentities,
};
use crate::cli_state::CliState;
use crate::cli_state::Result;

impl CliState {
    pub async fn secure_channels(
        &self,
        node_name: &str,
        pre_trusted_identities: Option<PreTrustedIdentities>,
    ) -> Result<Arc<SecureChannels>> {
        let identity_attributes_repository: Arc<dyn IdentityAttributesRepository> =
            self.identity_attributes_repository().await?;

        //TODO: fix this.  Either don't require it to be a bootstrappedidentitystore (and use the
        //trait instead),  or pass it from the general_options always.
        let identity_attributes_repository: Arc<dyn IdentityAttributesRepository> =
            Arc::new(match pre_trusted_identities {
                None => BootstrapedIdentityAttributesStore::new(
                    Arc::new(PreTrustedIdentities::new_from_string("{}")?),
                    identity_attributes_repository.clone(),
                ),
                Some(f) => BootstrapedIdentityAttributesStore::new(
                    Arc::new(f),
                    identity_attributes_repository.clone(),
                ),
            });

        debug!("create the secure channels service");
        let vault = self.get_node_vault(node_name).await?.vault().await?;
        let identities = Identities::create(self.database())
            .with_identity_attributes_repository(identity_attributes_repository)
            .with_vault(vault)
            .build();
        Ok(SecureChannels::builder()
            .await?
            .with_identities(identities)
            .build())
    }
}
