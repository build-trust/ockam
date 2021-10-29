use super::{NodeState, Router};
use crate::ShutdownType;
use ockam_core::Result;

pub(super) async fn exec(router: &mut Router, tt: ShutdownType) -> Result<()> {
    match tt {
        ShutdownType::Graceful(timeout) => graceful(router, timeout).await?,
        ShutdownType::Immediate => immediate(router).await?,
    }
    Ok(())
}

/// Implement the graceful shutdown strategy
async fn graceful(router: &mut Router, seconds: u8) -> Result<()> {
    Ok(())
}

/// Implement the immediate shutdown strategy
///
/// When triggering an `immediate` shutdown, all worker handles are
/// signalled to terminate, allowing workers to run their `async fn
/// shutdown(...)` hook.  However: the router will not wait for them!
/// Messages sent during the shutdown phase may not be delivered and
/// shutdown hooks may be suddenly interrupted by thread-deallocation.
async fn immediate(router: &mut Router) -> Result<()> {
    router.state.shutdown();
    router.map.internal.clear();
    Ok(())
}
