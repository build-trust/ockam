use ockam_command::enroll::enroll;

use crate::app::{
    configure_tauri_plugin_log, make_system_tray, process_application_event,
    process_system_tray_event, setup_app, AppState,
};
use crate::error::Result;

mod app;
mod enroll;
mod error;
mod quit;
mod tcp;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod others;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(target_os = "macos"))]
use others as platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    // For now, the application only consists of a system tray with several menu items
    let mut app = tauri::Builder::default()
        .plugin(configure_tauri_plugin_log())
        .setup(move |app| setup_app(app))
        .system_tray(make_system_tray(&app_state.options()))
        .on_system_tray_event(process_system_tray_event)
        .manage(app_state)
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application");

    platform::set_platform_activation_policy(&mut app);

    app.run(process_application_event);
}
