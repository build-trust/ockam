use crate::app::events::{SystemTrayOnUpdatePayload, SYSTEM_TRAY_ON_UPDATE};
use crate::app::{build_tray_menu, process_system_tray_menu_event};
use std::error::Error;
use std::mem::forget;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn;
use tauri::menu::Menu;
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager, Runtime};
use tracing::{debug, error, info};

/// Create the initial version of the system tray menu and the event listeners to update it.
pub fn setup<R: Runtime>(app: &mut App<R>) -> Result<(), Box<dyn Error>> {
    debug!("Setting up app");

    // Building the tray here rather than using the `tauri.conf.json` mechanism
    // to sidestep breaking changes the file format and consequent dependency
    // on a specific version of tauri-cli.
    TrayIconBuilder::with_id("tray")
        .tooltip("Ockam")
        .icon_as_template(true)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(process_system_tray_menu_event)
        .build(app)
        .expect("Couldn't initialize the system tray menu");

    // by design we have to keep our own reference to keep the menu alive
    let menu_holder = Arc::new(Mutex::new(None::<Menu<R>>));
    {
        let handle = app.handle().clone();
        let menu_holder = menu_holder.clone();
        app.listen_global(SYSTEM_TRAY_ON_UPDATE, move |event| {
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
                debug!("Building system tray menu");
                let tray = handle.tray().unwrap();
                let menu = build_tray_menu(&handle, payload).await;

                let old_menu = menu_holder.lock().unwrap().replace(menu.clone());
                tray.set_menu(Some(menu.clone()))
                    .expect("Couldn't update menu");

                // HACK: leak the previous menu to avoid a crash on macOS
                // TODO: remove me once tauri 2.0 stable is out
                forget(old_menu);
            });
        });
    }

    info!("App setup complete");
    Ok(())
}
