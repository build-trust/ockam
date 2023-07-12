use tauri::{Manager, SystemTray};
use tauri_plugin_log::{Target, TargetKind};

use crate::app::{process_application_event, process_system_tray_event, SystemTrayMenuBuilder};
use crate::ctx::TauriCtx;
use crate::error::Result;

mod app;
mod ctx;
mod enroll;
mod error;
mod quit;
mod tcp;

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
            let ctx = TauriCtx::new(app.app_handle());
            app.listen_global(app::events::SYSTEM_TRAY_ON_UPDATE, move |_event| {
                let _ = SystemTrayMenuBuilder::refresh(&ctx.clone());
            });
            app.trigger_global(app::events::SYSTEM_TRAY_ON_UPDATE, None);
            Ok(())
        })
        .system_tray(SystemTray::new().with_menu(SystemTrayMenuBuilder::default()))
        .on_system_tray_event(process_system_tray_event)
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
