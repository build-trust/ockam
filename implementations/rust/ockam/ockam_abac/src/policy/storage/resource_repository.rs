use crate::{Resource, ResourceName};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

/// This repository stores resources.
#[async_trait]
pub trait ResourcesRepository: Send + Sync + 'static {
    /// Store a policy for a given resource name and resource type
    async fn store_resource(&self, resource: &Resource) -> Result<()>;

    /// Return the policy associated to a given resource name and resource type
    async fn get_resource(&self, resource_name: &ResourceName) -> Result<Option<Resource>>;

    /// Delete all the entries for the given resource name
    async fn delete_resource(&self, resource_name: &ResourceName) -> Result<()>;
}

#[cfg(feature = "std")]
#[async_trait]
impl<T: ResourcesRepository> ResourcesRepository for AutoRetry<T> {
    async fn store_resource(&self, resource: &Resource) -> Result<()> {
        retry!(self.wrapped.store_resource(resource))
    }

    async fn get_resource(&self, resource_name: &ResourceName) -> Result<Option<Resource>> {
        retry!(self.wrapped.get_resource(resource_name))
    }

    async fn delete_resource(&self, resource_name: &ResourceName) -> Result<()> {
        retry!(self.wrapped.delete_resource(resource_name))
    }
}
