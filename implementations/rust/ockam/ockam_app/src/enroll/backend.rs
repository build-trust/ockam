use crate::{enroll, AppHandle};
use crate::{options, Result};

/// This trait represents all the enroll actions which
/// can interact with the network or the local configuration
/// It allows running the application with some mocks of those functions.
pub trait Backend {
    /// Trigger the authentication workflow for the current user
    fn enroll_user(&self, app_handle: AppHandle) -> Result<()>;

    /// Reset the local configuration as if running `ockam reset -y`
    fn reset(&self, app_handle: AppHandle) -> Result<()>;
}

pub struct DefaultBackend;

impl Backend for DefaultBackend {
    fn enroll_user(&self, app_handle: AppHandle) -> Result<()> {
        enroll::enroll_user::enroll_user(app_handle);
        Ok(())
    }

    fn reset(&self, app_handle: AppHandle) -> Result<()> {
        options::reset::reset(app_handle)
    }
}
