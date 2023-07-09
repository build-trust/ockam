use crate::ctx::TauriCtx;
use crate::enroll;
use crate::Result;

/// This trait represents all the enroll actions which
/// can interact with the network or the local configuration
/// It allows running the application with some mocks of those functions.
pub trait Backend {
    /// Trigger the authentication workflow for the current user
    fn enroll_user(&self) -> Result<()>;

    /// Reset the local configuration as if running `ockam reset -y`
    fn reset(&self, ctx: &TauriCtx) -> Result<()>;
}

pub struct DefaultBackend;

impl Backend for DefaultBackend {
    fn enroll_user(&self) -> Result<()> {
        enroll::enroll_user::enroll_user()
    }

    fn reset(&self, ctx: &TauriCtx) -> Result<()> {
        enroll::reset::reset(ctx)
    }
}
