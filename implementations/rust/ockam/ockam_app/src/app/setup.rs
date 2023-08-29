use crate::app::events::{SystemTrayOnUpdatePayload, SYSTEM_TRAY_ON_UPDATE};
use crate::app::{build_tray_menu, process_system_tray_event};
use std::error::Error;
use tauri::{async_runtime::spawn, App, Manager, SystemTray, Wry};
use tracing::{debug, error, info};

/// Create the initial version of the system tray menu and the event listeners to update it.
pub fn setup(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    debug!("Setting up app");

    let handle = app.handle();
    let tray_menu = tauri::async_runtime::block_on(build_tray_menu(&handle, None));
    SystemTray::new()
        .with_menu(tray_menu)
        .on_event(move |event| process_system_tray_event(&handle, event))
        .build(app)
        .expect("Couldn't initialize the system tray menu");

    let handle = app.handle();
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
        let handle = handle.clone();
        spawn(async move {
            handle
                .tray_handle()
                .set_menu(build_tray_menu(&handle, payload).await)
        });
    });
    info!("App setup complete");
    Ok(())
}
