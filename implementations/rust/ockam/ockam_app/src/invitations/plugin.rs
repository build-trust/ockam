use std::{sync::Arc, time::Duration};

use crate::app::events::system_tray_on_update;
use tauri::{
    async_runtime::{spawn, RwLock},
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tracing::{debug, error, info, trace};

use super::commands::*;
use super::events::{REFRESHED_INVITATIONS, REFRESH_INVITATIONS};
use super::state::InvitationState;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(30);

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("invitations")
        .invoke_handler(tauri::generate_handler![create_service_invitation,])
        .setup(|app, _api| {
            debug!("Initializing the invitations plugin");
            app.manage(Arc::new(RwLock::new(InvitationState::default())));

            let handle = app.clone();
            app.listen_global(REFRESH_INVITATIONS, move |_event| {
                let handle = handle.clone();
                spawn(async move {
                    refresh_invitations(handle.clone())
                        .await
                        .map_err(|e| error!(%e, "Failed to refresh invitations"))
                });
            });

            let handle = app.clone();
            spawn(async move {
                let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
                loop {
                    interval.tick().await;
                    trace!("refreshing invitations via background poll");
                    handle.trigger_global(REFRESH_INVITATIONS, None);
                }
            });

            let handle = app.clone();
            app.listen_global(REFRESHED_INVITATIONS, move |_event| {
                system_tray_on_update(&handle);
            });
            info!("Invitations plugin initialized");
            Ok(())
        })
        .build()
}
