use std::{sync::Arc, time::Duration};

use crate::app::events::system_tray_on_update;
use tauri::{
    async_runtime::{spawn, RwLock},
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tracing::{debug, error, info, trace};

use super::{
    commands::*,
    events::{REFRESHED_PROJECTS, REFRESH_PROJECTS},
    State,
};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60 * 5);

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("projects")
        .invoke_handler(tauri::generate_handler![list_projects])
        .setup(|app, _api| {
            debug!("Initializing the projects plugin");
            app.manage(Arc::new(RwLock::new(State::default())));

            let handle = app.clone();
            app.listen_global(REFRESH_PROJECTS, move |_event| {
                let handle = handle.clone();
                spawn(async move {
                    refresh_projects(handle.clone())
                        .await
                        .map_err(|e| error!(%e, "Failed to refresh projects"))
                });
            });
            let handle = app.clone();
            spawn(async move {
                let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
                loop {
                    interval.tick().await;
                    trace!("refreshing projects via background poll");
                    handle.trigger_global(REFRESH_PROJECTS, None);
                }
            });
            let handle = app.clone();
            app.listen_global(REFRESHED_PROJECTS, move |_event| {
                system_tray_on_update(&handle);
            });
            info!("Projects plugin initialized");
            Ok(())
        })
        .build()
}
