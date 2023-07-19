use std::error::Error;

use tauri::{App, Manager, Wry};

use crate::app::{events, AppState, SystemTrayMenuBuilder};

/// Set up the Tauri application so that it listens to tray update events
/// and rebuilds the system tray everytime there is a change
pub fn setup_app(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.app_handle();
    let app_state = app_handle.state::<AppState>();
    let options = app_state.options();
    app.listen_global(events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
        let app_handle = app_handle.clone();
        let options_clone = options.clone();
        tauri::async_runtime::spawn(async move {
            SystemTrayMenuBuilder::refresh(&app_handle, &options_clone).await
        });
    });
    Ok(())
}
