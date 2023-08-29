use std::time::Duration;

use crate::app::AppState;
use crate::shared_service::relay::create_relay;
use crate::shared_service::tcp_outlet::*;

use tauri::{
    async_runtime::spawn,
    plugin::{Builder, TauriPlugin},
    Manager, Wry,
};
use tracing::{debug, info, trace};

use super::events::CHECK_RELAY;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(120);

pub(crate) fn init() -> TauriPlugin<Wry> {
    Builder::new("shared_service")
        .invoke_handler(tauri::generate_handler![tcp_outlet_create])
        .setup(|app, _api| {
            debug!("Initializing the shared service plugin");
            let handle = app.clone();
            app.listen_global(CHECK_RELAY, move |_event| {
                let handle = handle.clone();
                spawn(async move {
                    let app_state = handle.state::<AppState>();
                    let _ = create_relay(&app_state).await;
                });
            });
            let handle = app.clone();
            spawn(async move {
                let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
                loop {
                    interval.tick().await;
                    trace!("checking relay via background poll");
                    handle.trigger_global(CHECK_RELAY, None);
                }
            });
            info!("Shared service plugin initialized");
            Ok(())
        })
        .build()
}
