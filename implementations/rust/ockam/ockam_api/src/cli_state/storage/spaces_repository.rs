use crate::cloud::space::Space;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

/// This trait supports the storage of spaces as retrieved from the Controller
///
///  - in addition to the space data, we can set a space as the default space
///  - a space is identified by its id by default when getting it or setting it as the default
///
#[async_trait]
pub trait SpacesRepository: Send + Sync + 'static {
    /// Store a space
    async fn store_space(&self, space: &Space) -> Result<()>;

    /// Return a space for a given id
    async fn get_space(&self, space_id: &str) -> Result<Option<Space>>;

    /// Return a space for a given name
    async fn get_space_by_name(&self, name: &str) -> Result<Option<Space>>;

    /// Return the list of all spaces
    async fn get_spaces(&self) -> Result<Vec<Space>>;

    /// Return the default space
    async fn get_default_space(&self) -> Result<Option<Space>>;

    /// Set a space as the default one
    async fn set_default_space(&self, space_id: &str) -> Result<()>;

    /// Delete a space
    async fn delete_space(&self, space_id: &str) -> Result<()>;
}

#[async_trait]
impl<T: SpacesRepository> SpacesRepository for AutoRetry<T> {
    async fn store_space(&self, space: &Space) -> Result<()> {
        retry!(self.wrapped.store_space(space))
    }

    async fn get_space(&self, space_id: &str) -> Result<Option<Space>> {
        retry!(self.wrapped.get_space(space_id))
    }

    async fn get_space_by_name(&self, name: &str) -> Result<Option<Space>> {
        retry!(self.wrapped.get_space_by_name(name))
    }

    async fn get_spaces(&self) -> Result<Vec<Space>> {
        retry!(self.wrapped.get_spaces())
    }

    async fn get_default_space(&self) -> Result<Option<Space>> {
        retry!(self.wrapped.get_default_space())
    }

    async fn set_default_space(&self, space_id: &str) -> Result<()> {
        retry!(self.wrapped.set_default_space(space_id))
    }

    async fn delete_space(&self, space_id: &str) -> Result<()> {
        retry!(self.wrapped.delete_space(space_id))
    }
}
