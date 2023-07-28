use crate::app::{configure_tauri_plugin_log, process_application_event, setup_app, AppState};
use crate::error::Result;
use shared_service::tcp_outlet::{tcp_outlet_close_window, tcp_outlet_create};

mod app;
mod enroll;
mod error;
#[cfg(feature = "invitations")]
mod invitations;
mod options;
mod platform;
mod shared_service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // For now, the application only consists of a system tray with several menu items
    #[cfg_attr(not(feature = "invitations"), allow(unused_mut))]
    let mut builder = tauri::Builder::default()
        .plugin(configure_tauri_plugin_log())
        .plugin(tauri_plugin_positioner::init())
        .setup(move |app| setup_app(app))
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            tcp_outlet_create,
            tcp_outlet_close_window
        ]);

    #[cfg(feature = "invitations")]
    {
        builder = builder.plugin(invitations::plugin::init());
    }

    let mut app = builder
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application");

    platform::set_platform_activation_policy(&mut app);

    app.run(process_application_event);
}
