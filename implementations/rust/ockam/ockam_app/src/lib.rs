use tauri::{Manager, SystemTray};
use tauri_plugin_log::{Target, TargetKind};

use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::app::{process_application_event, process_system_tray_event, SystemTrayMenuBuilder};
use crate::error::Result;

mod app;
mod enroll;
mod error;
mod quit;
mod tcp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args = GlobalArgs::default().set_quiet();
    let options = CommandGlobalOpts::new(args);
    let options_clone = options.clone();

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
        .setup(move |app| {
            let app_handle = app.app_handle().clone();
            app.listen_global(app::events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
                let _ = SystemTrayMenuBuilder::refresh(&app_handle, &options_clone);
            });
            app.trigger_global(app::events::SYSTEM_TRAY_ON_UPDATE, None);
            Ok(())
        })
        .system_tray(SystemTray::new().with_menu(SystemTrayMenuBuilder::default(&options)))
        .on_system_tray_event(process_system_tray_event)
        .manage(options)
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
