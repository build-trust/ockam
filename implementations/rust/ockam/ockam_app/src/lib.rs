use crate::app::{create_system_tray, process_application_event, process_system_tray_event};

mod app;
mod enroll;
mod quit;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let system_tray = create_system_tray();

    // For now the application only consists in a system tray with several menu items
    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(process_system_tray_event)
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
