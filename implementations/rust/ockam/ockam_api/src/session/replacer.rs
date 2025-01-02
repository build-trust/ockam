use std::time::Duration;

use ockam::remote::RemoteRelayInfo;
use ockam_core::{async_trait, Address, Result, Route};

//most sessions replacer are dependent on the node manager, if many session
//fails concurrently, which is the common scenario we need extra time
//to account for the lock contention
pub const MAX_RECOVERY_TIME: Duration = Duration::from_secs(30);
pub const MAX_CONNECT_TIME: Duration = Duration::from_secs(15);

#[async_trait]
pub trait SessionReplacer: Send + Sync + 'static {
    async fn create(&mut self) -> Result<ReplacerOutcome>;

    async fn close(&mut self);

    async fn on_session_down(&self);

    async fn on_session_replaced(&self);
}

#[async_trait]
pub trait AdditionalSessionReplacer: Send + Sync + 'static {
    async fn create_additional(&mut self) -> Result<Route>;
    async fn close_additional(&mut self, enable_fallback: bool);
}

#[derive(Debug, Clone)]
pub struct CurrentInletStatus {
    pub route: Route,
    pub worker: Option<Address>,
}

#[derive(Debug, Clone)]
pub enum ReplacerOutputKind {
    Inlet(CurrentInletStatus),
    Relay(RemoteRelayInfo),
}

#[derive(Debug, Clone)]
pub struct ReplacerOutcome {
    pub ping_route: Route,
    pub kind: ReplacerOutputKind,
}
