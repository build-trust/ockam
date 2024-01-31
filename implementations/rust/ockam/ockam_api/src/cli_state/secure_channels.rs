use std::sync::Arc;

use ockam::identity::{Identities, SecureChannels};

use crate::cli_state::CliState;
use crate::cli_state::Result;

impl CliState {
    pub async fn secure_channels(&self, node_name: &str) -> Result<Arc<SecureChannels>> {
        debug!("create the secure channels service");
        let vault = self.get_node_vault(node_name).await?.vault().await?;
        let identities = Identities::create(self.database())
            .with_vault(vault)
            .build();
        Ok(SecureChannels::from_identities(identities))
    }
}
