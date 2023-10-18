use crate::scheduler::ScheduledTask;
use crate::state::AppState;
use ockam_core::async_trait;
use tracing::warn;

pub(crate) struct RefreshProjectsTask {
    state: AppState,
}

impl RefreshProjectsTask {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ScheduledTask for RefreshProjectsTask {
    async fn run(&self) {
        let result = self.state.refresh_projects().await;
        if let Err(e) = result {
            warn!(%e, "Failed to refresh projects");
        }
    }
}

pub(crate) struct RefreshInvitationsTask {
    state: AppState,
}

impl RefreshInvitationsTask {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ScheduledTask for RefreshInvitationsTask {
    async fn run(&self) {
        let result = self.state.refresh_invitations().await;
        if let Err(e) = result {
            warn!(%e, "Failed to refresh invitations");
        }
    }
}

pub(crate) struct RefreshInletsTask {
    state: AppState,
}

impl RefreshInletsTask {
    pub(crate) fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl ScheduledTask for RefreshInletsTask {
    async fn run(&self) {
        let result = self.state.refresh_inlets().await;
        if let Err(e) = result {
            warn!(%e, "Failed to refresh inlets");
        }
    }
}
