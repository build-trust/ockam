use std::sync::Arc;

use tauri::{Manager, SystemTray};
use tauri_plugin_log::{Target, TargetKind};

use ockam::NodeBuilder;
use ockam_command::enroll::enroll;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

use crate::app::{
    process_application_event, process_system_tray_event, AppState, SystemTrayMenuBuilder,
};
use crate::error::Result;

mod app;
mod enroll;
mod error;
mod quit;
mod tcp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let options = CommandGlobalOpts::new(GlobalArgs::default().set_quiet());
    let (context, mut executor) = NodeBuilder::new().no_logging().build();
    tauri::async_runtime::set(context.runtime().clone());
    tauri::async_runtime::spawn(async move { executor.start_router().await });
    let app_state = AppState::new(Arc::new(context), options.clone());
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
            Ok(())
        })
        .system_tray(SystemTray::new().with_menu(SystemTrayMenuBuilder::default(&options)))
        .on_system_tray_event(process_system_tray_event)
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![])
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application")
        .run(process_application_event);
}
