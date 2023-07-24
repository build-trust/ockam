use std::error::Error;

use tauri::{App, Manager, SystemTray, Wry};

pub use app_state::*;
pub use logging::*;
pub use process::*;
pub use tray_menu::*;

mod app_state;
pub(crate) mod events;
mod logging;
mod process;
mod tray_menu;

/// Set up the Tauri application. This function is called once when the application starts.
///
/// Create the initial version of the system tray menu and the event listeners to update it.
pub fn setup_app(app: &mut App<Wry>) -> Result<(), Box<dyn Error>> {
    let state = app.state::<AppState>();

    // Setup tray menu
    let tray_menu = TrayMenu::default().build(state.is_enrolled());
    let moved_app = app.handle();
    SystemTray::new()
        .with_menu(tray_menu)
        .on_event(move |event| process_system_tray_event(&moved_app, event))
        .build(app)
        .expect("Couldn't initialize the system tray menu");

    // Setup event listeners
    let moved_app = app.handle();
    app.listen_global(events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
        let moved_app = moved_app.clone();
        tauri::async_runtime::spawn(async move {
            let app_state = moved_app.state::<AppState>();
            let mut tray_menu = TrayMenu::default();
            tray_menu.refresh(&moved_app, &app_state).await;
        });
    });
    Ok(())
}
