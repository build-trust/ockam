#[cfg(feature = "log")]
use crate::app::configure_tauri_plugin_log;
#[cfg(all(not(feature = "log"), feature = "tracing"))]
use crate::app::configure_tracing_log;
use crate::app::{process_application_event, setup_app, AppState};
use crate::cli::check_ockam_executable;
use crate::error::Result;
use shared_service::tcp_outlet::tcp_outlet_create;
use std::process::exit;

mod app;
mod cli;
mod enroll;
mod error;
#[cfg(feature = "invitations")]
mod invitations;
mod options;
mod platform;
#[cfg(feature = "invitations")]
mod projects;
mod shared_service;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(all(feature = "tracing", not(feature = "log")))]
    configure_tracing_log();

    // Exit if there is any issue with the ockam command
    // The log messages should explain what went wrong
    if check_ockam_executable().is_err() {
        exit(-1)
    }

    // For now, the application only consists of a system tray with several menu items
    #[cfg_attr(not(feature = "invitations"), allow(unused_mut))]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(move |app| setup_app(app))
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![tcp_outlet_create]);

    #[cfg(feature = "log")]
    {
        builder = builder.plugin(configure_tauri_plugin_log());
    }

    #[cfg(feature = "invitations")]
    {
        builder = builder.plugin(projects::plugin::init());
        builder = builder.plugin(invitations::plugin::init());
    }

    let mut app = builder
        .build(tauri::generate_context!())
        .expect("Error while building the Ockam application");

    platform::set_platform_activation_policy(&mut app);

    app.run(process_application_event);
}
