use std::{sync::Arc, time::Duration};

use tauri::{
    async_runtime::{spawn, RwLock},
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tracing::trace;

use super::commands::*;
use super::events::{REFRESHED_INVITATIONS, REFRESH_INVITATIONS};
use super::state::InvitationState;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("sharing")
        .invoke_handler(tauri::generate_handler![
            accept_invitation,
            create_service_invitation,
            list_invitations,
            refresh_invitations
        ])
        .setup(|app, _api| {
            app.manage(Arc::new(RwLock::new(InvitationState::default())));
            let handle = app.clone();
            app.listen_global(REFRESH_INVITATIONS, move |_event| {
                let handle = handle.clone();
                spawn(async move { refresh_invitations(handle.clone()).await });
            });
            let handle = app.clone();
            spawn(async move {
                handle.trigger_global(REFRESH_INVITATIONS, None);
                let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
                loop {
                    interval.tick().await;
                    trace!("refreshing invitations via background poll");
                    handle.trigger_global(REFRESH_INVITATIONS, None);
                }
            });
            let handle = app.clone();
            app.listen_global(REFRESHED_INVITATIONS, move |_event| {
                handle.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
            });
            Ok(())
        })
        .build()
}
