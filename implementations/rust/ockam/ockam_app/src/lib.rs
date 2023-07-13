use crate::app::{process_application_event, process_system_tray_event, SystemTrayMenuBuilder};
use crate::error::Result;
use tauri::{Manager, SystemTray, Wry};
use tauri_plugin_log::{Target, TargetKind};

mod app;
mod enroll;
mod error;
mod quit;
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
            let moved_app_handle = app.handle();
            SystemTray::new()
                .with_menu(SystemTrayMenuBuilder::full())
                .on_event(move |event| process_system_tray_event(moved_app_handle.clone(), event))
                .build(app)?;
            let moved_app_handle = app.handle();
            app.listen_global(app::events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
                let menu = SystemTrayMenuBuilder::full();
                let _ = moved_app_handle.tray_handle().set_menu(menu);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
