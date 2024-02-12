use super::Result;
use crate::CliState;
use ockam_abac::{Resource, ResourceName};

impl CliState {
    pub async fn store_resource(&self, resource: &Resource) -> Result<()> {
        self.resources_repository().store_resource(resource).await?;
        Ok(())
    }

    pub async fn delete_resource(&self, resource_name: &ResourceName) -> Result<()> {
        self.resources_repository()
            .delete_resource(resource_name)
            .await?;
        Ok(())
    }
}
