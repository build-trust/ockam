use crate::state::AppState;

impl AppState {
    pub(crate) async fn load_relay_model_state(&self) {
        self.create_relay(
            self.context(),
            self.state().await,
            self.node_manager().await,
        )
        .await;
    }
}
