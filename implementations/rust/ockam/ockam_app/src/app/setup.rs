use crate::app::events::{SystemTrayOnUpdatePayload, SYSTEM_TRAY_ON_UPDATE};
use crate::app::{build_tray_menu, process_system_tray_menu_event, AppState};
use std::error::Error;
use std::mem::forget;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::async_runtime::spawn;
use tauri::menu::Menu;
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager, Runtime};
use tracing::{debug, error, info};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Create the initial version of the system tray menu and the event listeners to update it.
pub fn setup<R: Runtime>(app: &mut App<R>) -> Result<(), Box<dyn Error>> {
    debug!("Setting up app");

    // Building the tray here rather than using the `tauri.conf.json` mechanism
    // to sidestep breaking changes the file format and consequent dependency
    // on a specific version of tauri-cli.
    TrayIconBuilder::with_id("tray")
        .tooltip("Ockam")
        .icon_as_template(true)
        .icon(
            app.default_window_icon()
                .expect("No default window icon")
                .clone(),
        )
        .on_menu_event(process_system_tray_menu_event)
        .build(app)
        .expect("Couldn't initialize the system tray menu");

    // by design we have to keep our own reference to keep the menu alive
    let menu_holder = Arc::new(Mutex::new(None::<Menu<R>>));
    {
        let handle = app.handle().clone();
        let menu_holder = menu_holder.clone();
        app.listen_global(SYSTEM_TRAY_ON_UPDATE, move |event| {
            let app_state = handle.state::<AppState>();
            let event_tracker = app_state.debounce_event(&handle, SYSTEM_TRAY_ON_UPDATE);
            if event_tracker.is_processing() {
                return;
            }

            let payload = match event.payload() {
                Some(p) => match SystemTrayOnUpdatePayload::try_from(p) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        error!(?e, "Couldn't deserialize payload");
                        None
                    }
                },
                None => None,
            };

            let menu_holder = menu_holder.clone();
            let handle = handle.clone();
            spawn(async move {
                let _event_tracker = event_tracker;

                debug!("Building system tray menu");
                let tray = match handle.tray() {
                    Some(t) => t,
                    None => {
                        error!("Couldn't get tray menu handler");
                        return;
                    }
                };

                let menu = build_tray_menu(&handle, payload).await;

                let mut lock = match menu_holder.lock() {
                    Ok(l) => l,
                    Err(e) => {
                        error!(?e, "Couldn't get menu lock");
                        return;
                    }
                };
                if let Some(old_menu) = lock.replace(menu.clone()) {
                    // HACK: leak the previous menu to avoid a crash on macOS
                    // TODO: remove me once tauri 2.0 stable is out
                    forget(old_menu);
                }

                let _ = tray
                    .set_menu(Some(menu.clone()))
                    .map_err(|_| error!("Couldn't update menu"));
            });
        });
    }

    // Update the tray menu frequently once the user is enrolled.
    let handle = app.handle().clone();
    spawn(async move {
        let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
        loop {
            interval.tick().await;
            let app_state = handle.state::<AppState>();
            if app_state.is_enrolled().await.unwrap_or(false) {
                debug!("Refreshing tray menu via background poll");
                handle.trigger_global(SYSTEM_TRAY_ON_UPDATE, None);
            }
        }
    });

    info!("App setup complete");
    Ok(())
}
