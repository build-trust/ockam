use std::sync::Arc;

use ockam::identity::{Identities, SecureChannelSqlxDatabase, SecureChannels};

use crate::cli_state::CliState;
use crate::cli_state::Result;

impl CliState {
    pub async fn secure_channels_for_node(&self, node_name: &str) -> Result<Arc<SecureChannels>> {
        debug!("create the secure channels service for node {node_name}");
        let named_vault = self.get_node_vault(node_name).await?;
        let vault = self.make_vault(named_vault).await?;
        let identities = Identities::create_with_node(self.database(), node_name)
            .with_vault(vault)
            .build();
        Ok(SecureChannels::from_identities(
            identities,
            SecureChannelSqlxDatabase::make_repository(self.database()),
        ))
    }

    pub async fn secure_channels(&self) -> Result<Arc<SecureChannels>> {
        debug!("create the secure channels service");
        let named_vault = self.get_or_create_default_named_vault().await?;
        let vault = self.make_vault(named_vault).await?;
        let identities = Identities::create(self.database())
            .with_vault(vault)
            .build();
        Ok(SecureChannels::from_identities(
            identities,
            SecureChannelSqlxDatabase::make_repository(self.database()),
        ))
    }
}
