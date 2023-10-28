use crate::state::AppState;
use tracing::debug;

impl AppState {
    pub(crate) async fn load_relay_model_state(&self) {
        debug!("Loading relay model state");
        self.create_relay(
            self.context(),
            self.state().await,
            self.node_manager().await,
        )
        .await;
    }
}
