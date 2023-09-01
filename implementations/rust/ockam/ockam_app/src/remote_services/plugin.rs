use std::{sync::Arc, time::Duration};

use crate::app::events::system_tray_on_update;
use tauri::{
    async_runtime::{spawn, RwLock},
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tracing::{debug, error, info, trace};

use super::commands::*;
use super::events::{REFRESHED_REMOTE_SERVICES, REFRESH_REMOTE_SERVICES};
use super::state::RemoteServicesState;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("remote_services")
        .setup(|app, _api| {
            debug!("Initializing the remote_services plugin");
            app.manage(Arc::new(RwLock::new(RemoteServicesState::default())));

            let handle = app.clone();
            app.listen_global(REFRESH_REMOTE_SERVICES, move |_event| {
                let handle = handle.clone();
                spawn(async move {
                    refresh_remote_services(handle.clone())
                        .await
                        .map_err(|e| error!(%e, "Failed to refresh invitations"))
                });
            });

            let handle = app.clone();
            spawn(async move {
                let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
                loop {
                    interval.tick().await;
                    trace!("refreshing remote services via background poll");
                    handle.trigger_global(REFRESH_REMOTE_SERVICES, None);
                }
            });

            let handle = app.clone();
            app.listen_global(REFRESHED_REMOTE_SERVICES, move |_event| {
                system_tray_on_update(&handle);
            });
            info!("Remote services plugin initialized");
            Ok(())
        })
        .build()
}
