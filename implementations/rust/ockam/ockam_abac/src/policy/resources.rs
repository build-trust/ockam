use crate::{Resource, ResourceName, ResourcesRepository};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

#[derive(Clone)]
pub struct Resources {
    resources_repository: Arc<dyn ResourcesRepository>,
}

impl Resources {
    pub fn new(resources_repository: Arc<dyn ResourcesRepository>) -> Self {
        Self {
            resources_repository,
        }
    }

    pub async fn store_resource(&self, resource: &Resource) -> Result<()> {
        self.resources_repository.store_resource(resource).await?;
        Ok(())
    }

    pub async fn delete_resource(&self, resource_name: &ResourceName) -> Result<()> {
        self.resources_repository
            .delete_resource(resource_name)
            .await?;
        Ok(())
    }
}
