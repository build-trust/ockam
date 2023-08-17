use std::error::Error;

use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager, Runtime};

pub use app_state::*;
pub use logging::*;
pub use model_state::*;
pub use process::*;
pub use tray_menu::*;

mod app_state;
pub(crate) mod events;
mod logging;
mod model_state;
mod model_state_repository;
mod process;
mod tray_menu;

/// Set up the Tauri application. This function is called once when the application starts.
///
/// Create the initial version of the system tray menu and the event listeners to update it.
pub fn setup_app<R: Runtime>(app: &mut App<R>) -> Result<(), Box<dyn Error>> {
    let app_handle = app.handle().clone();
    let menu = tauri::async_runtime::block_on(build_tray_menu(&app_handle));

    TrayIconBuilder::with_id("tray")
        .tooltip("Ockam")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(process_system_tray_menu_event)
        .build(app)
        .expect("Couldn't initialize the system tray menu");

    // Setup event listeners
    let app_handle = app.handle().clone();
    app.listen_global(events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
        let app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            app_handle
                .tray()
                .unwrap()
                .set_menu(Some(build_tray_menu(&app_handle).await.clone()))
        });
    });
    Ok(())
}
