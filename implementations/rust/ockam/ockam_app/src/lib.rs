use crate::app::tray_menu::process_system_tray_event;
use crate::app::{process_application_event, State};
use crate::error::Result;
use tauri::{Manager, SystemTray, Wry};
use tauri_plugin_log::{Target, TargetKind};

mod app;
mod enroll;
mod error;
mod options;
mod tcp;

type AppHandle = tauri::AppHandle<Wry>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // For now the application only consists in a system tray with several menu items
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("ockam.log".to_string()),
                    }),
                ])
                .build(),
        )
        .setup(|app| {
            // Setup tray menu
            let tray_menu = {
                let app_handle = app.handle();
                let state = app_handle.state::<State>();
                let tray_menu = state.tray_menu.read().unwrap();
                tray_menu.init().build()
            };
            let moved_app_handle = app.handle();
            SystemTray::new()
                .with_menu(tray_menu)
                .on_event(move |event| process_system_tray_event(moved_app_handle.clone(), event))
                .build(app)?;
            Ok(())
        })
        .manage(State::default())
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
